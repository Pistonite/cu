
type ValidatorFn = Box<dyn FnMut(&mut String) -> cu::Result<bool>>;

/// Create a new prompt builder with the given message.
pub fn prompt(message: impl Into<String>) -> PromptBuilder {
    PromptBuilder {
        message: message.into(),
        validator: None,
    }
}


/// Initial prompt builder (can call `yesno()`, `password()`, or use directly)
pub struct PromptBuilder {
    message: String,
    validator: Option<ValidatorFn>,
}

impl PromptBuilder {
    /// Convert to a yes/no prompt.
    pub fn yesno(self) -> PromptBuilderYesNo {
        todo!()
    }

    /// Convert to a password (hidden input) prompt.
    pub fn password(self) -> PromptBuilderPassword {
        todo!()
    }

    /// Set a default value to use if the prompt is cancelled.
    pub fn if_cancel(self, default: cu::ZString) -> PromptBuilderIfCancel<Self, cu::ZString> {
        todo!()
    }

    /// Add a validation function to the prompt.
    pub fn validate<F>(self, validator: F) -> Self
    where
        F: FnMut(&mut String) -> crate::Result<bool> + 'static,
    {
        todo!()
    }

    /// Wait synchronously for the prompt result.
    pub fn wait(self) -> cu::Result<Option<cu::ZString>> {
        todo!()
    }

    /// Wait asynchronously for the prompt result.
    pub async fn co_wait(self) -> cu::Result<Option<cu::ZString>> {
        todo!()
    }
}

// ========== PromptBuilderYesNo ==========

/// Yes/no prompt builder (returns `bool`)
pub struct PromptBuilderYesNo {
    message: String,
    validator: Option<ValidatorFn>,
}

impl PromptBuilderYesNo {
    /// Set a default value to use if the prompt is cancelled.
    pub fn if_cancel(self, default: bool) -> PromptBuilderIfCancel<Self, bool> {
        todo!()
    }

    /// Add a validation function to the prompt.
    pub fn validate<F>(self, validator: F) -> Self
    where
        F: FnMut(&mut String) -> crate::Result<bool> + 'static,
    {
        todo!()
    }

    /// Wait synchronously for the prompt result.
    pub fn wait(self) -> cu::Result<Option<bool>> {
        todo!()
    }

    /// Wait asynchronously for the prompt result.
    pub async fn co_wait(self) -> cu::Result<Option<bool>> {
        todo!()
    }
}

// ========== PromptBuilderPassword ==========

/// Password (hidden input) prompt builder (returns `ZString`)
pub struct PromptBuilderPassword {
    message: String,
    validator: Option<ValidatorFn>,
}

impl PromptBuilderPassword {
    /// Set a default value to use if the prompt is cancelled.
    pub fn if_cancel(self, default: cu::ZString) -> PromptBuilderIfCancel<Self, cu::ZString> {
        todo!()
    }

    /// Add a validation function to the prompt.
    pub fn validate<F>(self, validator: F) -> Self
    where
        F: FnMut(&mut String) -> crate::Result<bool> + 'static,
    {
        todo!()
    }

    /// Wait synchronously for the prompt result.
    pub fn wait(self) -> cu::Result<Option<cu::ZString>> {
        todo!()
    }

    /// Wait asynchronously for the prompt result.
    pub async fn co_wait(self) -> cu::Result<Option<cu::ZString>> {
        todo!()
    }
}

// ========== PromptBuilderIfCancel ==========

/// Prompt builder with a default value on cancel (returns `Output` directly)
pub struct PromptBuilderIfCancel<Inner, Output> {
    inner: Inner,
    default: Output,
}

impl<Inner, Output> PromptBuilderIfCancel<Inner, Output> {
    /// Add a validation function to the prompt.
    pub fn validate<F>(self, validator: F) -> Self
    where
        F: FnMut(&mut String) -> crate::Result<bool> + 'static,
    {
        todo!()
    }

    /// Wait synchronously for the prompt result.
    pub fn wait(self) -> cu::Result<Output> {
        todo!()
    }

    /// Wait asynchronously for the prompt result.
    pub async fn co_wait(self) -> cu::Result<Output> {
        todo!()
    }
}
