use std::collections::HashMap;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::media;

pub type Id = i64;

#[derive(Serialize, Deserialize, Clone)]
pub struct Product {
    id: Id,
    name: String,
    description: String,
    icon: Option<media::Id>,
}

impl Product {
    fn new(id: Id, name: String, description: String, icon: Option<media::Id>) -> Self {
        Product {
            id,
            name,
            description,
            icon,
        }
    }

    fn update(&mut self, partial: PartialProduct) {
        if let Some(fresh) = partial.id {
            self.id = fresh;
        }
        if let Some(fresh) = partial.name.clone() {
            self.name = fresh;
        }
        if let Some(fresh) = partial.description.clone() {
            self.description = fresh;
        }
        if let Some(fresh) = partial.icon {
            let _ = self.icon.insert(fresh);
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PartialProduct {
    id: Option<Id>,
    name: Option<String>,
    description: Option<String>,
    icon: Option<media::Id>,
}

impl PartialProduct {
    pub fn icon(&self) -> Option<Uuid> {
        self.icon
    }
}

pub struct Products {
    products: HashMap<Id, Product>,
}

impl Products {
    pub fn new() -> Self {
        Products {
            products: HashMap::new(),
        }
    }

    pub fn create(
        &mut self,
        name: String,
        description: String,
        icon: Option<media::Id>,
    ) -> Product {
        let id = self.generate_id();
        let product = Product::new(id, name, description, icon);
        self.products.insert(id, product.clone());
        product
    }

    pub fn update(&mut self, id: Id, partial: PartialProduct) -> anyhow::Result<Product> {
        let mut product = self.find(id)?;
        if partial.id.filter(|&updated| updated != id).is_some() {
            self.delete(id)?;
        }
        product.update(partial);
        self.products.insert(product.id, product.clone());
        Ok(product)
    }

    pub fn delete(&mut self, id: Id) -> anyhow::Result<Product> {
        self.products.remove(&id).ok_or_else(|| self.no_product(id))
    }

    pub fn all(&self) -> Vec<Product> {
        self.products.values().cloned().collect()
    }

    pub fn find(&self, id: Id) -> anyhow::Result<Product> {
        self.products
            .get(&id)
            .cloned()
            .ok_or_else(|| self.no_product(id))
    }

    pub fn set_icon(&mut self, id: Id, icon: media::Id) -> anyhow::Result<Product> {
        self.update(
            id,
            PartialProduct {
                id: None,
                name: None,
                description: None,
                icon: Some(icon),
            },
        )
    }

    pub fn icon(&mut self, id: Id) -> anyhow::Result<media::Id> {
        self.find(id)
            .and_then(|product| product.icon.ok_or_else(|| self.no_icon(id)))
    }

    fn no_product(&self, id: Id) -> anyhow::Error {
        anyhow!("No product with id {id}")
    }

    fn no_icon(&self, id: Id) -> anyhow::Error {
        anyhow!("Product {id} has no icon")
    }

    fn generate_id(&self) -> Id {
        self.products
            .keys()
            .max()
            .map(|id| id + 1)
            .unwrap_or_default()
    }
}
