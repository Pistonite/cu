use cu::pre::*;

/// Deserializable regex
#[derive(Clone)]
pub struct Regex {
    inner: regex::Regex,

    #[cfg(feature = "serde")]
    lit: String,
}
impl Regex {
    pub fn new(s: &str) -> cu::Result<Self> {
        let inner = cu::check!(regex::Regex::new(s), "failed to build regex from {s:?}")?;
        #[cfg(feature = "serde")]
        let s = Self {
            inner,
            lit: s.to_string(),
        };
        #[cfg(not(feature = "serde"))]
        let s = Self { inner };
        Ok(s)
    }
}
impl std::ops::Deref for Regex {
    type Target = regex::Regex;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl std::ops::DerefMut for Regex {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl std::fmt::Display for Regex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.lit)
    }
}
impl std::fmt::Debug for Regex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Regex({})", self.inner)
    }
}
impl Regex {
    /// Get the string that this regex was created from
    pub fn to_str(&self) -> &str {
        &self.lit
    }
}
impl From<Regex> for regex::Regex {
    fn from(value: Regex) -> Self {
        value.inner
    }
}

#[cfg(feature = "serde")]
impl Serialize for Regex {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.lit)
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
                    Ok(x) => Ok(Regex {
                        inner: x,
                        lit: v.to_string(),
                    }),
                }
            }
        }
    }
}
