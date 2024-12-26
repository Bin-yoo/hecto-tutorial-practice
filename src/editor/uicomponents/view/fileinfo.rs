use std::{
    fmt::{self, Display},
    path::{Path, PathBuf},
};

#[derive(Default, Debug)]
pub struct FileInfo {
    path: Option<PathBuf>,
}

impl FileInfo {
    pub fn from(file_name: &str) -> Self {
        Self {
            path: Some(PathBuf::from(file_name)),
        }
    }

    /// 获取文件路径引用
    pub fn get_path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// 获取路径是否存在bool
    pub const fn has_path(&self) -> bool {
        self.path.is_some()
    }
}

impl Display for FileInfo {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self
            .get_path()
            // 然后获取文件名
            .and_then(|path| path.file_name())
            // 转成str
            .and_then(|name| name.to_str())
            .unwrap_or("[No Name]");
        write!(formatter, "{name}")
    }
}