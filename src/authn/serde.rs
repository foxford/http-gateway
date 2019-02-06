use std::fmt;

use jsonwebtoken::Algorithm;
use serde::de::{Deserializer, Error, Unexpected, Visitor};

struct AlgorithmVisitor;

impl<'de> Visitor<'de> for AlgorithmVisitor {
    type Value = Algorithm;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "a name of signature or MAC algorithm specified in RFC7518: JSON Web Algorithms (JWA)"
        )
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        use std::str::FromStr;

        Algorithm::from_str(v).map_err(|_| Error::invalid_value(Unexpected::Str(v), &self))
    }
}

pub(crate) fn algorithm<'de, D>(deserializer: D) -> Result<Algorithm, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(AlgorithmVisitor)
}

struct FileVisitor;

impl<'de> Visitor<'de> for FileVisitor {
    type Value = Vec<u8>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a path to an existing file")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        use std::fs::File;
        use std::io::Read;

        let mut data = Vec::new();
        File::open(v)
            .and_then(|mut file| file.read_to_end(&mut data).and_then(|_| Ok(data)))
            .map_err(|_| Error::invalid_value(Unexpected::Str(v), &self))
    }
}

pub(crate) fn file<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(FileVisitor)
}
