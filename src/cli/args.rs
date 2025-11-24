pub trait Validatable {
    fn from_id(id: usize) -> Self;
    fn from_string(s: String) -> Self;
    fn get_string(&self) -> Option<&str>;
}

#[derive(Clone)]
pub enum Args {
    Id(usize),
    Script(String),
}

#[derive(Clone)]
pub enum Item {
    Id(usize),
    Name(String),
}

impl Validatable for Args {
    fn from_id(id: usize) -> Self {
        Args::Id(id)
    }
    fn from_string(s: String) -> Self {
        Args::Script(s)
    }

    fn get_string(&self) -> Option<&str> {
        match self {
            Args::Id(_) => None,
            Args::Script(s) => Some(s),
        }
    }
}

impl Validatable for Item {
    fn from_id(id: usize) -> Self {
        Item::Id(id)
    }
    fn from_string(s: String) -> Self {
        Item::Name(s)
    }

    fn get_string(&self) -> Option<&str> {
        match self {
            Item::Id(_) => None,
            Item::Name(s) => Some(s),
        }
    }
}

pub fn validate<T: Validatable>(s: &str) -> Result<T, String> {
    if let Ok(id) = s.parse::<usize>() {
        Ok(T::from_id(id))
    } else {
        Ok(T::from_string(s.to_owned()))
    }
}
