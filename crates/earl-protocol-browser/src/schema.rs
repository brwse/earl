// placeholder

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

#[derive(Debug, Clone)]
pub struct BrowserOperationTemplate;

#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct BrowserStep;
