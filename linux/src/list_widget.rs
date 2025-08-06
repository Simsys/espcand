use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, List, ListItem},
    style::Stylize,
};
use std::collections::VecDeque;
use smol::io::{AsyncWrite, Error};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct ListWidget<const CAP: usize> {
    content: VecDeque<String>,
    border_title: &'static str,
}

impl<const CAP: usize> ListWidget<CAP> {
    pub fn new(border_title: &'static str) -> Self {
        let content = VecDeque::<String>::with_capacity(CAP); 
        Self { content, border_title }
    }

    pub fn render(&mut self, frame: &mut Frame, area: &Rect) {
        let line_count = (area.height - 2) as usize;
        let start_idx = if self.content.len() > line_count {
            self.content.len() - line_count
        } else {
            0
        };
        let content: Vec<ListItem> = self.content
            .range(start_idx..)
            .map(|s| {
                let mut item = ListItem::new(s.clone());
                if s.as_bytes().len() >= 3 {
                    if &s.as_bytes()[..3] == b"<= " {
                        item = item.green();
                    }
                    if &s.as_bytes()[..3] == b"=> " {
                        item = item.yellow();
                    }
                    if &s.as_bytes()[..3] == b"$rf" {
                        item = item.blue();
                    }
                }
                if s.as_bytes().len() >= 7 {
                    if &s.as_bytes()[..7] == b"=> $err" {
                        item = item.red();
                    }
                }
                item
            })
            .collect();
        let content = List::new(content).block(Block::bordered().title(self.border_title));
        frame.render_widget(content, *area);
    }

    pub fn add_item(&mut self, item: String) {
        if self.content.len() == CAP {
            let _ = self.content.pop_front();
        }
        self.content.push_back(item);
    }
}

#[derive(Debug)]
pub struct ListWidgets<const CAP:usize> {
    can_widget: ListWidget<CAP>,
    cmd_widget: ListWidget<CAP>,
}

impl<const CAP:usize> ListWidgets<CAP> {
    pub fn new() -> Self {
        Self {
            can_widget: ListWidget::new(" Received CAN Messages "),
            cmd_widget: ListWidget::new(" Commands and Messages "),
        }
    }

    pub fn can(&mut self) -> &mut ListWidget<CAP> {
        &mut self.can_widget
    }

    pub fn cmd(&mut self) -> &mut ListWidget<CAP> {
        &mut self.cmd_widget
    }

    pub fn render(&mut self, frame: &mut Frame, can_area: &Rect, cmd_area: &Rect) {
        self.can_widget.render(frame, can_area);
        self.cmd_widget.render(frame, cmd_area);
    }
}

impl<const CAP:usize> AsyncWrite for &mut ListWidgets<CAP> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {        
        let n = buf.len();
        let mut s = String::new();
        for c in buf {
            if *c != b'\n' {
                if let Some(ch) = char::from_u32(*c as u32) {
                    s.push(ch);
                } 
            } else {
                if s.len() >= 3 && &s.as_str()[..3] == "$rf" {
                    self.can_widget.add_item(s.clone());
                } else {
                    let s = format!("=> {}", &s.as_str());
                    self.cmd_widget.add_item(s);
                }
                s.clear();
            }
        }
        Poll::Ready(Ok(n))    
    }
    
    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))    
    }

    fn poll_close(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))    
    }
}
