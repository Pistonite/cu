// $ -qq
// $ -q
// $
// $ -v
// $ -vv
// $ --color=always -qq
// $ --color=always -q
// $ --color=always
// $ --color=always -v
// $ --color=always -vv

#[cu::cli]
fn main(_: cu::cli::Flags) -> cu::Result<()> {
    cu::info!(
        "this is an info messagenmultilineaa 你好 sldkfjals🤖kdjflkasjdflkjasldkfjaklsdjflkjasldkfjlaksjdflkajsdklfjlaksjdfkljasldkfjlasldkjflaskdjflaksjdlfkajsldkfjkasjdlfkjaskldjflajsdlkfjlaskjdfklajsdf"
    );
    cu::warn!("this is a warn message\n");
    cu::error!("this is error message\n\n");
    cu::debug!("this is debug message\n2\n\n");
    cu::trace!("this is trace message\n\n2\n");
    cu::print!("today's weather is {}", "good");
    cu::hint!("today's weather is {}", "ok");
    Ok(())
}
