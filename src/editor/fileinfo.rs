use std::{
    fmt::{self, Display},
    path::PathBuf,
};

#[derive(Default, Debug, Clone)]
pub struct FileInfo {
    pub path: Option<PathBuf>,
}

impl FileInfo {
    pub fn from(file_name: &str) -> Self {
        Self {
            path: Some(PathBuf::from(file_name)),
        }
    }
}

impl Display for FileInfo {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self
            .path
            // 获取引用
            .as_ref()
            // 然后获取文件名
            .and_then(|path| path.file_name())
            // 转成str
            .and_then(|name| name.to_str())
            .unwrap_or("[No Name]");
        write!(formatter, "{name}")
    }
}