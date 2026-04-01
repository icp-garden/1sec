use super::{config, forest, head};

#[derive(Debug)]
pub struct State {
    pub forest: forest::State,
    pub head: head::State,
}

impl State {
    pub fn new(config: config::Config) -> Self {
        Self {
            forest: forest::State::new(),
            head: head::State::new(config.head),
        }
    }
}
