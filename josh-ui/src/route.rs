use super::*;

#[derive(Switch, Clone, PartialEq)]
#[to = "/~/{*:mode}/{*:repo}@{*:rev}({*:filter})/{*:path}[{*:meta}]"]
pub struct AppRoute {
    pub mode: String,
    pub repo: String,
    pub rev: String,
    pub filter: String,
    pub path: String,
    pub meta: String,
}

impl AppRoute {
    pub fn with_path(&self, path: &str) -> Self {
        let mut s = self.clone();
        s.path = path.to_string();
        return s;
    }

    pub fn path_up(&self) -> Self {
        let mut s = self.clone();
        let p = std::path::PathBuf::from(self.path.clone());
        s.path = p
            .parent()
            .map(|x| x.to_string_lossy().to_string())
            .unwrap_or_default();

        return s;
    }

    pub fn edit_filter(&self) -> Self {
        let mut s = self.clone();
        s.mode = "filter".to_string();
        return s;
    }

    pub fn filename(&self) -> String {
        let p = std::path::PathBuf::from(self.path.clone());
        p.file_name()
            .map(|x| x.to_string_lossy().to_string())
            .unwrap_or_default()
    }

    pub fn breadcrumbs(&self) -> Vec<Self> {
        let mut r = vec![];
        let mut x = self.clone();

        loop {
            if x.path != "" {
                r.push(x.clone());
            } else {
                break;
            }
            x = x.path_up();
        }
        return r;
    }

    pub fn with_filter(&self, filter: &str) -> Self {
        let mut s = self.clone();
        s.mode = "browse".to_string();
        s.filter = filter.to_string();
        return s;
    }

    pub fn with_rev(&self, rev: &str) -> Self {
        let mut s = self.clone();
        s.rev = rev.to_string();
        return s;
    }
}

pub type AppAnchor = yew_router::components::RouterAnchor<AppRoute>;
