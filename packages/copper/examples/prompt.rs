use pistonite_cu as cu;

use cu::pre::*;

// can only manually test this because password not prompted from stdin
#[cu::cli]
fn main(_: cu::cli::Flags) -> cu::Result<()> {
    // test cases:
    // n,    # cancel
    // ^C,   # cancel
    // y,    # pass
    if !cu::yesno!("do you want to continue?")? {
        return Ok(());
    }
    cu::info!("user picked yes");

    // test cases:
    // ^C  # cancel
    // test ("hi, test")
    // ("hi, ")
    let name = cu::prompt!("please enter your name")?;
    cu::info!("hi, {name}");

    // test cases:
    // ^C ("foobar")
    // "" ("")
    // "123" ("123")
    let password = cu::prompt("please enter your password")
        .password()
        .if_cancel("foobar")
        .run()?;
    cu::info!("user entered: {password}");

    // test cases
    // "asdf" ("try again"), javascript # error
    // ^C,        # cancel
    // "asdf", ^C # cancel
    // "asdf", "asdf" ("try again"), "rust" # pass
    let expected = "rust";
    let answer = cu::prompt(format!(
        "what's your favorite programming language? please answer {}",
        expected
    ))
    .validate_with(move |answer| {
        if answer == expected {
            return Ok(true);
        }
        if answer == "javascript" {
            cu::bail!("that's not good");
        }
        cu::error!("try again");
        Ok(false)
    })
    .run()?;
    let answer = cu::check!(answer, "user cancelled")?;
    cu::ensure!(*answer == expected)?;

    // test cases
    // "not number" # loop
    // "-1"         # loop
    // "6"          # loop
    // "3"          # pass
    let mut index: i32 = 0;
    cu::prompt("select a number between 0 and 5")
        .or_cancel()
        .validate_with(|answer| {
            let number = match cu::parse::<i32>(answer) {
                Err(e) => {
                    cu::error!("{e}");
                    cu::hint!("please ensure you are entering a number");
                    return Ok(false);
                }
                Ok(x) => x,
            };
            if number < 0 {
                cu::error!("the number you entered is too small");
                return Ok(false);
            }
            if number > 5 {
                cu::error!("the number you entered is too big");
                return Ok(false);
            }
            index = number;
            Ok(true)
        })
        .run()?;
    cu::info!("index is {index}");

    // test cases
    // "" # too short
    // "asdfasdfasdfasdfasdf" # too long
    // "foo foo foo" # illegal
    // "123456" # error
    // "helloworld" # pass
    let password = cu::prompt(
        "please enter a password between 8 and 16 charactres and only contains sensible characters",
    )
    .password()
    .or_cancel()
    .validate_with(|answer| {
        if answer == "123456" {
            cu::bail!("how can you do that, bye");
        }
        if answer.len() < 8 {
            cu::error!("password is too short");
            return Ok(false);
        }
        if answer.len() > 16 {
            cu::error!("password is too long");
            return Ok(false);
        }
        if let Err(e) = cu::password_chars_legal(answer) {
            cu::error!("invalid password: {e}");
            return Ok(false);
        }
        Ok(true)
    })
    .run()?;
    cu::print!("{password}");
    Ok(())
}
