use cu::pre::*;

/// Deserializable regex
#[derive(Clone)]
pub struct Regex(regex::Regex, String);
impl std::ops::Deref for Regex {
    type Target = regex::Regex;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Regex {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl std::fmt::Display for Regex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::fmt::Debug for Regex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Regex({})", self.0)
    }
}
impl Regex {
    /// Get the string that this regex was created from
    pub fn to_str(&self) -> &str {
        &self.1
    }
}
impl From<Regex> for regex::Regex {
    fn from(value: Regex) -> Self {
        value.0
    }
}

#[cfg(feature = "serde")]
impl Serialize for Regex {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.1)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Regex {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        return d.deserialize_str(Visitor);
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Regex;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a regular expression")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                match regex::Regex::new(v) {
                    Err(e) => Err(serde::de::Error::custom(format!(
                        "invalid regular expression '{v}': {e}"
                    ))),
                    Ok(x) => Ok(Regex(x, v.to_string())),
                }
            }
        }
    }
}
