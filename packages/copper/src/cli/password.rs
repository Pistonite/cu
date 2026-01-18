macro_rules! special_chars {
    ($c1:literal | $($c:literal)|* $(|)?) => {
        static LEGAL_PASSWORD_ERROR_MESSAGE: &str = concat!(
            "password contains illegal characters, only ascii alphanumeric characters and special characters in the following list are allowed: ",
            stringify!($c1),
            $( ", ", stringify!($c), )*
        );
        #[inline(always)]
        fn special_char_legal(c: char) -> bool {
            matches!(c, $c1 $( | $c )* )
        }
    }
}
special_chars! { '!' | '#' | '$' | '%' | '&' | '(' | ')' | '*' | '+' | ',' | '-' | '.' | '/' | ':' | ';' | '<' | '=' | '>' | '?' | '@' | '[' | ']' | '^' | '_' | '`' | '{' | '|' | '}' | '~'}
/// Check if the password contains all "legal" characters (and is non-empty)
pub fn password_chars_legal(s: &str) -> crate::Result<()> {
    if s.is_empty() {
        crate::bail!("password cannot be empty");
    }
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || special_char_legal(c))
    {
        return Ok(());
    }
    crate::bail!("{LEGAL_PASSWORD_ERROR_MESSAGE}");
}
