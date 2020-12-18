use git2;
use tracing;

use super::empty_tree;
use super::filter_cache;
use super::filters;
use super::UnapplyFilter;
use std::collections::HashSet;

fn all_equal(a: git2::Parents, b: &[&git2::Commit]) -> bool {
    let a: Vec<_> = a.collect();
    if a.len() != b.len() {
        return false;
    }

    for (x, y) in b.iter().zip(a.iter()) {
        if x.id() != y.id() {
            return false;
        }
    }
    return true;
}

// takes everything from base except it's tree and replaces it with the tree
// given
pub fn rewrite(
    repo: &git2::Repository,
    base: &git2::Commit,
    parents: &[&git2::Commit],
    tree: &git2::Tree,
) -> super::JoshResult<git2::Oid> {
    if base.tree()?.id() == tree.id() && all_equal(base.parents(), parents) {
        // Looks like an optimization, but in fact serves to not change the commit in case
        // it was signed.
        return Ok(base.id());
    }

    return Ok(repo.commit(
        None,
        &base.author(),
        &base.committer(),
        &base.message_raw().unwrap_or("no message"),
        tree,
        parents,
    )?);
}

fn find_original(
    repo: &git2::Repository,
    bm: &mut std::collections::HashMap<git2::Oid, git2::Oid>,
    spec: &str,
    contained_in: git2::Oid,
    filtered: git2::Oid,
) -> super::JoshResult<git2::Oid> {
    if contained_in == git2::Oid::zero() {
        return Ok(git2::Oid::zero());
    }
    if let Some(original) = bm.get(&filtered) {
        return Ok(*original);
    }
    if let Some(oid) =
        super::filter_cache::lookup_forward(&repo, &spec, contained_in)
    {
        bm.insert(contained_in, oid);
    }
    let mut walk = repo.revwalk()?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL)?;
    walk.push(contained_in)?;

    for original in walk {
        let original = original?;
        if Some(filtered)
            == super::filter_cache::lookup_forward(&repo, spec, original)
        {
            bm.insert(filtered, original);
            return Ok(original);
        }
    }

    return Ok(git2::Oid::zero());
}

#[tracing::instrument(skip(repo))]
pub fn unapply_filter(
    repo: &git2::Repository,
    filterobj: &dyn filters::Filter,
    unfiltered_old: git2::Oid,
    old: git2::Oid,
    new: git2::Oid,
) -> super::JoshResult<UnapplyFilter> {
    let walk = {
        let mut walk = repo.revwalk()?;
        walk.set_sorting(git2::Sort::REVERSE | git2::Sort::TOPOLOGICAL)?;
        walk.push(new)?;
        if let Ok(_) = walk.hide(old) {
            tracing::trace!("walk: hidden {}", old);
        } else {
            tracing::warn!("walk: can't hide");
        }
        walk
    };

    let mut bm = std::collections::HashMap::new();
    let mut ret =
        find_original(&repo, &mut bm, &filterobj.spec(), unfiltered_old, new)?;
    for rev in walk {
        let rev = rev?;

        let s = tracing::span!(tracing::Level::TRACE, "walk commit", ?rev);
        let _e = s.enter();

        let module_commit = repo.find_commit(rev)?;

        if bm.contains_key(&module_commit.id()) {
            continue;
        }

        let filtered_parent_ids: Vec<_> = module_commit.parent_ids().collect();

        let original_parents: std::result::Result<Vec<_>, _> =
            filtered_parent_ids
                .iter()
                .map(|x| -> super::JoshResult<_> {
                    find_original(
                        &repo,
                        &mut bm,
                        &filterobj.spec(),
                        unfiltered_old,
                        *x,
                    )
                })
                .filter(|x| {
                    if let Ok(i) = x {
                        *i != git2::Oid::zero()
                    } else {
                        true
                    }
                })
                .map(|x| -> super::JoshResult<_> { Ok(repo.find_commit(x?)?) })
                .collect();

        tracing::info!(
            "parents: {:?} -> {:?}",
            original_parents,
            filtered_parent_ids
        );

        let original_parents = original_parents?;

        let original_parents_refs: Vec<&git2::Commit> =
            original_parents.iter().collect();

        let tree = module_commit.tree()?;

        let commit_message =
            module_commit.summary().unwrap_or("NO COMMIT MESSAGE");

        let new_trees: super::JoshResult<HashSet<_>> = {
            let s = tracing::span!(
                tracing::Level::TRACE,
                "unapply filter",
                ?commit_message,
                ?rev,
                ?filtered_parent_ids,
                ?original_parents_refs
            );
            let _e = s.enter();
            original_parents_refs
                .iter()
                .map(|x| -> super::JoshResult<_> {
                    Ok(filterobj.unapply(&repo, tree.clone(), x.tree()?)?.id())
                })
                .collect()
        };

        let new_trees = match new_trees {
            Ok(new_trees) => new_trees,
            Err(super::JoshError(msg)) => {
                return Err(super::josh_error(&format!(
                    "\nCan't apply {:?} ({:?})\n{}",
                    commit_message,
                    module_commit.id(),
                    msg
                )))
            }
        };

        let new_tree = match new_trees.len() {
            1 => repo.find_tree(
                *new_trees
                    .iter()
                    .next()
                    .ok_or(super::josh_error("iter.next"))?,
            )?,
            0 => {
                tracing::debug!("unrelated history");
                // 0 means the history is unrelated. Pushing it will fail if we are not
                // dealing with either a force push or a push with the "josh-merge" option set.
                filterobj.unapply(&repo, tree, empty_tree(&repo))?
            }
            parent_count => {
                // This is a merge commit where the parents in the upstream repo
                // have differences outside of the current filter.
                // It is unclear what base tree to pick in this case.
                tracing::warn!("rejecting merge");
                return Ok(UnapplyFilter::RejectMerge(parent_count));
            }
        };

        ret =
            rewrite(&repo, &module_commit, &original_parents_refs, &new_tree)?;
        bm.insert(module_commit.id(), ret);
    }

    tracing::trace!("done {:?}", ret);
    return Ok(UnapplyFilter::Done(ret));
}

#[tracing::instrument(skip(repo, transaction))]
fn transform_commit(
    repo: &git2::Repository,
    filterobj: &dyn filters::Filter,
    from_refsname: &str,
    to_refname: &str,
    transaction: &mut filter_cache::Transaction,
) -> super::JoshResult<usize> {
    let mut updated_count = 0;
    if let Ok(reference) = repo.revparse_single(&from_refsname) {
        let original_commit = reference.peel_to_commit()?;

        let filter_commit = super::filters::apply_filter_cached(
            &repo,
            &*filterobj,
            original_commit.id(),
        )?;

        transaction.insert(
            &filterobj.spec(),
            original_commit.id(),
            filter_commit,
        );

        let previous = repo
            .revparse_single(&to_refname)
            .map(|x| x.id())
            .unwrap_or(git2::Oid::zero());

        if filter_commit != previous {
            updated_count += 1;
            tracing::trace!(
                "transform_commit: update reference: {:?} -> {:?}, target: {:?}, filter: {:?}",
                &from_refsname,
                &to_refname,
                filter_commit,
                &filterobj.spec()
            );
        }

        if filter_commit != git2::Oid::zero() {
            ok_or!(
                repo.reference(
                    &to_refname,
                    filter_commit,
                    true,
                    "apply_filter"
                )
                .map(|_| ()),
                {
                    tracing::error!(
                        "can't update reference: {:?} -> {:?}, target: {:?}, filter: {:?}",
                        &from_refsname,
                        &to_refname,
                        filter_commit,
                        &filterobj.spec()
                    );
                }
            );
        }
    } else {
        tracing::warn!(
            "transform_commit: Can't find reference {:?}",
            &from_refsname
        );
    };
    return Ok(updated_count);
}

#[tracing::instrument(skip(repo))]
pub fn apply_filter_to_refs(
    repo: &git2::Repository,
    filterobj: &dyn filters::Filter,
    refs: &[(String, String)],
) -> super::JoshResult<usize> {
    rs_tracing::trace_scoped!("apply_filter_to_refs","spec":filterobj.spec());
    let mut transaction = super::filter_cache::Transaction::new();

    let mut updated_count = 0;
    for (k, v) in refs {
        updated_count +=
            transform_commit(&repo, &*filterobj, &k, &v, &mut transaction)?;
    }
    return Ok(updated_count);
}
