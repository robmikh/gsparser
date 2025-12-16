#[derive(Debug)]
pub struct ResourceItem<'a> {
    pub key: &'a str,
    pub value: ResourceItemValue<'a>,
}

#[derive(Debug)]
pub enum ResourceItemValue<'a> {
    Single(&'a str),
    Collection(ResourceItemCollection<'a>),
}

#[derive(Debug)]
pub struct ResourceItemCollection<'a>(pub Vec<ResourceItem<'a>>);

impl<'a> ResourceItemValue<'a> {
    pub fn as_single(&self) -> Option<&'a str> {
        match self {
            Self::Single(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_collection(&self) -> Option<&ResourceItemCollection<'a>> {
        match self {
            Self::Collection(collection) => Some(collection),
            _ => None,
        }
    }
}

impl<'a> ResourceItemCollection<'a> {
    pub fn get(&self, key: &str) -> Option<&ResourceItem<'a>> {
        self.0.iter().find(|x| x.key == key)
    }
}

fn parse_key_value_pair<'a>(line: &'a str) -> Option<(&'a str, &'a str)> {
    let whitespace_first = line.find(|x: char| x.is_whitespace());
    if let Some(whitespace_first) = whitespace_first {
        let offset = whitespace_first + 1;
        let rest_of_line = &line[offset..];
        let whitespace_end = offset + rest_of_line.find(|x: char| !x.is_whitespace()).unwrap();
        let key = &line[..whitespace_first].trim_matches('"');
        let value = &line[whitespace_end..].trim_matches('"');
        Some((*key, *value))
    } else {
        None
    }
}

pub fn parse_resource_item<'a>(lines: &mut std::str::Lines<'a>) -> Option<ResourceItem<'a>> {
    // TODO: Properly parse file
    while let Some(line) = lines.next() {
        //println!("LINE: {}", line);
        let line = line.trim();
        if line.starts_with("//") || line.is_empty() {
            continue;
        }

        let item_key;
        let item_value;

        if let Some((key, value)) = parse_key_value_pair(line) {
            //println!("\"{}\" : \"{}\"", key, value);
            item_key = Some(key);
            item_value = Some(ResourceItemValue::Single(value));
        } else {
            if line == "}" {
                return None;
            }

            let key = line.trim_matches('"');
            let open_bracket = lines.next()?.trim();
            if key == "{" || key == "}" || open_bracket != "{" {
                // TODO: Return errors
                println!("Malformed resource file!");
                return None;
            }
            item_key = Some(key);

            let mut collection = Vec::new();
            while let Some(item) = parse_resource_item(lines) {
                collection.push(item);
            }

            item_value = Some(ResourceItemValue::Collection(ResourceItemCollection(
                collection,
            )));
        }

        let key = item_key?;
        let value = item_value?;

        return Some(ResourceItem { key, value });
    }
    None
}

pub trait ParseResource: Sized {
    fn parse(resource: &ResourceItem) -> Option<Self>;
}

pub trait ParseResourceValue: Sized {
    fn parse(resource: &ResourceItem) -> Option<Self>;
    fn default_value() -> Option<Self>;
}

impl ParseResourceValue for String {
    fn parse(resource: &ResourceItem) -> Option<Self> {
        let value = resource.value.as_single()?;
        Some(value.to_owned())
    }
    fn default_value() -> Option<Self> {
        None
    }
}

impl ParseResourceValue for bool {
    fn parse(resource: &ResourceItem) -> Option<Self> {
        let value = resource.value.as_single()?;
        if let Ok(value) = value.parse::<i32>() {
            Some(value != 0)
        } else {
            None
        }
    }
    fn default_value() -> Option<Self> {
        None
    }
}

impl<T: ParseResourceValue> ParseResourceValue for Option<T> {
    fn parse(resource: &ResourceItem) -> Option<Option<T>> {
        let value = T::parse(resource);
        Some(value)
    }
    fn default_value() -> Option<Self> {
        Some(None)
    }
}

#[macro_export]
macro_rules! resource_struct {
    ($resource_name:ident { $( ($key_name:literal) $field_name:ident : $field_ty:ty),* $(,)* }) => {
        #[allow(dead_code)]
        #[derive(Debug)]
        pub struct $resource_name {
            $(
                pub $field_name : $field_ty,
            )*
        }

        impl crate::resource::ParseResource for $resource_name {
            fn parse(resource: &crate::resource::ResourceItem) -> Option<Self> {
                use crate::resource::ParseResourceValue;

                $(
                    let mut $field_name: Option< $field_ty > = < $field_ty >::default_value();
                )*

                let collection = resource.value.as_collection()?;
                for item in &collection.0 {
                    match item.key {
                        $(
                            $key_name => {
                                $field_name = < $field_ty >::parse(item);
                            }
                        )*
                        // TODO: Return error with struct?
                        _ => {
                            println!("Unknown key: \"{}\"", item.key);
                            return None;
                        },
                    }
                }

                $(
                    let $field_name: $field_ty = if let Some($field_name) = $field_name {
                        $field_name
                    } else {
                        println!("Missing \"{}\"", $key_name);
                        return None;
                    };
                )*

                Some(Self {
                    // TODO: Return errors
                    $(
                        $field_name,
                    )*
                })
            }
        }
    };
}
