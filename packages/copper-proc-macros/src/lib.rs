use pm::pre::*;

/// **For the Command Line Interface feature set,
/// please refer to the [`cu::cli`](../pistonite-cu/cli/index.html) module.**
///
/// This is the documentation for the `#[cu::cli]` macro.
///
/// By annotating the main function, this macro generates
/// a shim that will reference the `cu::cli::Flags` command line
/// arguments and initialize logging, printing, and prompting
/// systems accordingly.
///
/// The main function can be async or sync. It should
/// return a `cu::Result`
/// ```rust,ignore
/// #[cu::cli]
/// fn main(flags: cu::cli::Flags) -> cu::Result<()> {
///     cu::debug!("flags are {flags:?}");
///     Ok(())
/// }
/// ```
///
/// To also define your own flags using the [`clap`](https://docs.rs/clap)
/// crate, define a CLI struct that derives `clap::Parser`.
/// Note the prelude import (`cu::pre::*`) automatically
/// brings `clap` into scope. You don't even need to add it
/// to `Cargo.toml`!
///
/// Make sure to `#[clap(flatten)]` the flags into your struct.
///
/// ```rust,ignore
/// # use pistonite_cu as cu;
/// use cu::pre::*;
/// /// My program
/// ///
/// /// This is my program, it is very good.
/// #[derive(clap::Parser, Clone)]
/// struct Args {
///     /// Input of the program
///     #[clap(short, long)]
///     input: String,
///     /// Output of the program
///     #[clap(short, long)]
///     output: Option<String>,
///     #[clap(flatten)]
///     inner: cu::cli::Flags,
/// }
/// ```
/// Now, to tell `cu` where to look for the flags,
/// specify the name of the field with `flags = "field"`
/// ```rust,ignore
/// // use the flags attribute to refer to the cu::cli::Flags field inside the Args struct
/// #[cu::cli(flags = "inner")]
/// fn main(args: Args) -> cu::Result<()> {
///     cu::info!("input is {}", args.input);
///     cu::info!("output is {:?}", args.output);
///     Ok(())
/// }
/// ```
///
/// Alternatively, implement `AsRef<cu::cli::Flag>` for your struct.
///
/// ```rust,ignore
/// # use pistonite_cu as cu;
/// use cu::pre::*;
/// #[derive(clap::Parser, Clone)]
/// struct Args {
///     input: String,
///     #[clap(flatten)]
///     inner: cu::cli::Flags,
/// }
/// impl AsRef<cu::cli::Flags> for Args {
///     fn as_ref(&self) -> cu::cli::Flags {
///         &self.inner
///     }
/// }
/// #[cu::cli]
/// fn main(_: Args) -> cu::Result<()> {
///     Ok(())
/// }
/// ```
///
/// Or enable the `derive` feature and derive `AsRef` (via [`derive_more`](https://docs.rs/derive_more)).
/// ```rust,ignore
/// # use pistonite_cu as cu;
/// use cu::pre::*;
/// #[derive(clap::Parser, Clone, AsRef)]
/// struct Args {
///     input: String,
///     #[clap(flatten)]
///     #[as_ref]
///     inner: cu::cli::Flags,
/// }
/// #[cu::cli]
/// fn main(_: Args) -> cu::Result<()> {
///     Ok(())
/// }
/// ```
///
/// The attribute can also take a `preprocess` function
/// to process flags before initializing the CLI system.
/// This can be useful to merge multiple Flags instance
/// in the CLI. Note that the logging/printing system
/// will not work during the preprocess.
///
/// ```rust,ignore
/// # use pistonite_cu as cu;
/// use cu::pre::*;
///
/// #[derive(clap::Parser)]
/// struct Args {
///     #[clap(subcommand)]
///     subcommand: Option<Command>,
///     #[clap(flatten)]
///     inner: cu::cli::Flags,
/// }
/// impl Args {
///     fn preprocess(&mut self) {
///         // merge subcommand flags into top level flags
///         // this way, both `-v foo` and `foo -v` will work
///         if let Some(Command::Foo(c)) = &self.subcommand {
///             self.inner.merge(c);
///         }
///     }
/// }
/// impl AsRef<cu::cli::Flags> for Args {
///     fn as_ref(&self) -> &cu::cli::Flags {
///         &self.inner
///     }
/// }
/// #[derive(clap::Subcommand)]
/// enum Command {
///     Foo(cu::cli::Flags),
/// }
/// #[cu::cli(preprocess = Args::preprocess)]
/// fn main(args: Args) -> cu::Result<()> {
///     Ok(())
/// }
/// ```
///
#[proc_macro_attribute]
pub fn cli(attr: TokenStream, input: TokenStream) -> TokenStream {
    pm::flatten(cli::expand(attr, input))
}
mod cli;

/// Derive the [`cu::Parse`](../pistonite-cu/trait.Parse.html) trait
#[proc_macro_derive(Parse)]
pub fn derive_parse(input: TokenStream) -> TokenStream {
    pm::flatten(derive_parse::expand(input))
}
mod derive_parse;

/// Attribute macro for wrapping a function with an error context
///
/// See the [tests](https://github.com/Pistonite/cu/blob/main/packages/copper/tests/context.rs)
/// for examples
#[proc_macro_attribute]
pub fn context(attr: TokenStream, input: TokenStream) -> TokenStream {
    pm::flatten(context::expand(attr, input))
}
mod context;
