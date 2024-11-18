use std::{io::Error, fs::read_to_string};

#[derive(Default)]
pub struct Buffer {
    pub lines: Vec<String>
}

impl Buffer {

    // 读取文件内容到buffer中
    pub fn load(file_name: &str) -> Result<Self, Error> {
        let contents = read_to_string(file_name)?;
        let lines = contents.lines()
            .map(|value| String::from(value))
            .collect();

        Ok(Self{ lines })
    }

    // buffer是否为空
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}