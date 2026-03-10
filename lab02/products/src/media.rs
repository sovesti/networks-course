use std::collections::HashMap;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type Id = Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct Image {
    id: Id,
    raw: Vec<u8>,
}

impl Image {
    pub fn new(id: Id, raw: Vec<u8>) -> Self {
        Self { id, raw }
    }

    pub fn bytes(self) -> Vec<u8> {
        self.raw
    }
}

pub struct Images {
    images: HashMap<Id, Image>,
}

impl Images {
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
        }
    }

    pub fn create(&mut self, raw: Vec<u8>) -> Id {
        let id = self.generate_id();
        self.images.insert(id, Image::new(id, raw));
        id
    }

    pub fn find(&self, id: Id) -> anyhow::Result<Image> {
        self.images
            .get(&id)
            .cloned()
            .ok_or_else(|| self.no_image(id))
    }

    pub fn contains(&self, id: Id) -> anyhow::Result<()> {
        self.images
            .contains_key(&id)
            .then(|| ())
            .ok_or_else(|| self.no_image(id))
    }

    fn no_image(&self, id: Id) -> anyhow::Error {
        anyhow!("No image with id {id}")
    }

    fn generate_id(&self) -> Id {
        Uuid::new_v4()
    }
}
