use std::any::Any;


/// Try to get info from a panic payload
pub fn best_effort_panic_info<'a>(payload: &'a Box<dyn Any + Send + 'static>) -> &'a str {
    if let Some(s) = payload.downcast_ref::<&str>() {
        *s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.as_str()
    } else {
        crate::debug!("encountered unknown panic info with type id: {:?}", payload.type_id());
        "unknown panic info"
    }
}
