use super::{documentstatus::DocumentStatus, terminal::{Size, Terminal}};

pub struct StatusBar {
    // 当前保存状态
    current_status: DocumentStatus,
    // 是否需要重新渲染
    needs_redraw: bool,
    // 底部距离
    margin_bottom: usize,
    // 宽度
    width: usize,
    // 光标/当前操作位置
    position_y: usize,
    // 是否显示
    is_visible: bool,
}

impl StatusBar {

    // 
    pub fn new(margin_bottom: usize) -> Self {
        let size = Terminal::size().unwrap_or_default();
        let mut status_bar = Self {
            current_status: DocumentStatus::default(),
            needs_redraw: true,
            margin_bottom,
            width: size.width,
            position_y: 0,
            is_visible: false
        };

        status_bar.resize(size);

        status_bar
    }

    //
    pub fn resize(&mut self, size: Size) {
        self.width = size.width;
        // 初始化光标位置为0，默认状态下状态栏不可见
        let mut position_y = 0;
        let mut is_visible = false;
        // 检查是否新尺寸的高度减去底部距离后至少还剩一行用于显示状态栏
        if let Some(result) = size
            .height
            // 确保不会发生下溢（高度 - 底部距离）
            .checked_sub(self.margin_bottom)
            // 再减一确保有空间给状态栏
            .and_then(|result| result.checked_sub(1))
        {
            // 如果上述检查通过，则更新状态栏的位置和可见性
            position_y = result;
            is_visible = true;
        }
        self.position_y = position_y;
        self.is_visible = is_visible;
        self.needs_redraw = true;
    }

    // 更新状态
    pub fn update_status(&mut self, new_status: DocumentStatus) {
        if new_status != self.current_status {
            self.current_status = new_status;
            self.needs_redraw = true;
        }
    }

    // 渲染状态栏
    pub fn render(&mut self) {
        if !self.needs_redraw || !self.is_visible {
            return;
        }
        if let Ok(size) = Terminal::size() {
            // 组装状态栏的第一部分：文件名、行数和是否修改的指示符
            let line_count = self.current_status.line_count_to_string();
            let modified_indicator = self.current_status.modified_indicator_to_string();
            let beginning = format!(
                "{} - {line_count} {modified_indicator}",
                self.current_status.file_name
            );

            // 组装整个状态栏，在末尾加上位置指示符
            let position_indicator = self.current_status.position_indicator_to_string();
            // 计算剩余空间的长度，确保状态栏内容不会超出终端宽度
            let remainder_len = size.width.saturating_sub(beginning.len());
            // 使用格式化字符串将所有部分组合起来，确保位置指示符靠右对齐
            let status = format!("{beginning}{position_indicator:>remainder_len$}");
            
            // 只有当状态栏内容完全适合终端宽度时才打印；否则，打印空字符串以清除该行
            let to_print = if status.len() <= size.width {
                status
            } else {
                String::new()
            };
            // 在指定的位置打印倒置颜色的状态栏行
            let result = Terminal::print_inverted_row(self.position_y, &to_print);
            
            debug_assert!(result.is_ok(), "Failed to render status bar");
            self.needs_redraw = false;
        }
    }
}