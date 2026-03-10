use actix_form_data::{Field, Form, FormData, Value};
use actix_web::HttpRequest;
use futures_util::StreamExt;

pub struct UploadedIcon(pub Value<Vec<u8>>);

impl UploadedIcon {
    pub fn bytes(self) -> Option<Vec<u8>> {
        self.0
            .map()
            .and_then(|mut form| form.remove("icon"))
            .and_then(Value::file)
            .map(|file| file.result)
    }
}

impl FormData for UploadedIcon {
    type Item = Vec<u8>;
    type Error = actix_form_data::Error;

    fn form(_: &HttpRequest) -> Result<Form<Self::Item, Self::Error>, Self::Error> {
        Ok(Form::new().field(
            "icon",
            Field::file(async move |_, _, mut stream| {
                let mut bytes = vec![];
                while let Some(result) = stream.next().await {
                    if let Ok(chunk) = result {
                        bytes.push(chunk);
                    } else {
                        break;
                    }
                }
                Ok(bytes.concat())
            }),
        ))
    }

    fn extract(value: Value<Self::Item>) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(UploadedIcon(value))
    }
}
