  $ . ${TESTDIR}/setup_test_env.sh
  $ cd ${TESTTMP}


  $ git clone -q http://localhost:8001/real_repo.git
  warning: You appear to have cloned an empty repository.

  $ curl -s http://localhost:8002/version
  Version: 0.3.0

  $ cd real_repo

  $ git status
  On branch master
  
  No commits yet
  
  nothing to commit (create/copy files and use "git add" to track)

  $ git checkout -b master
  Switched to a new branch 'master'


  $ echo content1 > file1 1> /dev/null
  $ git add .
  $ git commit -m "initial" 1> /dev/null

  $ git checkout -b new1
  Switched to a new branch 'new1'
  $ echo content > newfile1 1> /dev/null
  $ git add .
  $ git commit -m "add newfile1" 1> /dev/null

  $ git checkout master 1> /dev/null
  Switched to branch 'master'
  $ echo content > newfile_master 1> /dev/null
  $ git add .
  $ git commit -m "newfile master" 1> /dev/null

  $ git merge new1 --no-ff
  Merge made by the 'recursive' strategy.
   newfile1 | 0
   1 file changed, 0 insertions(+), 0 deletions(-)
   create mode 100644 newfile1


  $ mkdir -p sub1/subsub
  $ echo contents1 > sub1/subsub/file1
  $ git add .
  $ git commit -m "add file1" 1> /dev/null

  $ mkdir sub2
  $ echo contents1 > sub2/file2
  $ git add sub2
  $ git commit -m "add file2" 1> /dev/null

  $ git sync
  * refs/heads/master -> refs/heads/master
  Pushing to http://localhost:8001/real_repo.git
  POST git-receive-pack (1435 bytes)
  updating local tracking ref 'refs/remotes/origin/master'
  

  $ cd ${TESTTMP}
  $ curl -s http://localhost:8002/flush
  Flushed credential cache
  $ git clone -q http://localhost:8002/real_repo.git:workspace=ws.git ws
  warning: You appear to have cloned an empty repository.
  $ cd ${TESTTMP}/ws
  $ cat > workspace.josh <<EOF
  > a/b = :/sub2
  > c = :/sub1
  > EOF

  $ git add .
  $ git commit -m "add workspace" 1> /dev/null
  $ git sync origin HEAD:refs/heads/master -o merge
  * HEAD -> refs/heads/master
  From http://localhost:8002/real_repo.git:workspace=ws
   * branch            4a199f3a19a292e6639dede0f8602afc19a82dfc -> FETCH_HEAD
  HEAD is now at 4a199f3 Merge from :workspace=ws
  Pushing to http://localhost:8002/real_repo.git:workspace=ws.git
  POST git-receive-pack (417 bytes)
  remote: warning: ignoring broken ref refs/namespaces/request_*/HEAD         (glob)
  remote: josh-proxy        
  remote: response from upstream:        
  remote:  To http://localhost:8001/real_repo.git        
  remote:    5d605ce..98c996c  JOSH_PUSH -> master        
  remote: REWRITE(1b46698f32d1d1db1eaeb34f8c9037778d65f3a9 -> 4a199f3a19a292e6639dede0f8602afc19a82dfc)        
  remote: 
  remote: 
  updating local tracking ref 'refs/remotes/origin/master'
  

  $ curl -s http://localhost:8002/flush
  Flushed credential cache
  $ git pull --rebase
  From http://localhost:8002/real_repo.git:workspace=ws
   + 1b46698...4a199f3 master     -> origin/master  (forced update)
  Already up to date.
  Current branch master is up to date.

  $ tree
  .
  |-- a
  |   `-- b
  |       `-- file2
  |-- c
  |   `-- subsub
  |       `-- file1
  `-- workspace.josh
  
  4 directories, 3 files

  $ git log --graph --pretty=%s
  *   Merge from :workspace=ws
  |\  
  | * add file2
  | * add file1
  * add workspace

  $ cd ${TESTTMP}/real_repo
  $ git pull --rebase
  From http://localhost:8001/real_repo
     5d605ce..98c996c  master     -> origin/master
  Updating 5d605ce..98c996c
  Fast-forward
   ws/workspace.josh | 2 ++
   1 file changed, 2 insertions(+)
   create mode 100644 ws/workspace.josh
  Current branch master is up to date.

  $ git log --graph --pretty=%s
  *   Merge from :workspace=ws
  |\  
  | * add workspace
  * add file2
  * add file1
  *   Merge branch 'new1'
  |\  
  | * add newfile1
  * | newfile master
  |/  
  * initial

  $ cd ${TESTTMP}/ws

  $ cat > workspace.josh <<EOF
  > a/c = :/sub2
  > c = :/sub1
  > EOF

  $ git add workspace.josh
  $ git commit -m "mod workspace" 1> /dev/null

  $ git log --graph --pretty=%s
  * mod workspace
  *   Merge from :workspace=ws
  |\  
  | * add file2
  | * add file1
  * add workspace


  $ git sync
    refs/heads/master -> refs/heads/master
  From http://localhost:8002/real_repo.git:workspace=ws
   * branch            891a8d3448a28a6c04405ca4dcf585dd0825cebd -> FETCH_HEAD
  HEAD is now at 891a8d3 mod workspace
  Pushing to http://localhost:8002/real_repo.git:workspace=ws.git
  POST git-receive-pack (421 bytes)
  remote: josh-proxy        
  remote: response from upstream:        
  remote:  To http://localhost:8001/real_repo.git        
  remote:    98c996c..6fde6f1  JOSH_PUSH -> master        
  remote: REWRITE(06230e638c617eeac83330c8c3ff980ba7b7d838 -> 891a8d3448a28a6c04405ca4dcf585dd0825cebd)        
  remote: 
  remote: 
  updating local tracking ref 'refs/remotes/origin/master'
  

  $ curl -s http://localhost:8002/flush
  Flushed credential cache
  $ git pull --rebase
  From http://localhost:8002/real_repo.git:workspace=ws
   + 06230e6...891a8d3 master     -> origin/master  (forced update)
  Already up to date.
  Current branch master is up to date.

  $ tree
  .
  |-- a
  |   |-- b
  |   |   `-- file2
  |   `-- c
  |       `-- file2
  |-- c
  |   `-- subsub
  |       `-- file1
  `-- workspace.josh
  
  5 directories, 4 files

  $ git log --graph --pretty=%s
  * mod workspace
  *   Merge from :workspace=ws
  |\  
  | * add file2
  | * add file1
  * add workspace

  $ cd ${TESTTMP}/real_repo

  $ curl -s http://localhost:8002/flush
  Flushed credential cache
  $ git pull --rebase
  From http://localhost:8001/real_repo
     98c996c..6fde6f1  master     -> origin/master
  Updating 98c996c..6fde6f1
  Fast-forward
   ws/a/b/file2      | 1 +
   ws/workspace.josh | 2 +-
   2 files changed, 2 insertions(+), 1 deletion(-)
   create mode 100644 ws/a/b/file2
  Current branch master is up to date.

  $ tree
  .
  |-- file1
  |-- newfile1
  |-- newfile_master
  |-- sub1
  |   `-- subsub
  |       `-- file1
  |-- sub2
  |   `-- file2
  `-- ws
      |-- a
      |   `-- b
      |       `-- file2
      `-- workspace.josh
  
  6 directories, 7 files
  $ git log --graph --pretty=%s
  * mod workspace
  *   Merge from :workspace=ws
  |\  
  | * add workspace
  * add file2
  * add file1
  *   Merge branch 'new1'
  |\  
  | * add newfile1
  * | newfile master
  |/  
  * initial

