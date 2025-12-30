use pistonite_cu as cu;

#[test]
fn test_example1() {
    let msg = format!("{:?}", example1(42).unwrap_err());
    assert_eq!(
        msg,
        r"failed with arg 42

Caused by:
    example1"
    )
}

#[cu::error_ctx("failed with arg {arg}")]
fn example1(arg: u32) -> cu::Result<()> {
    cu::bail!("example1")
}

#[test]
fn test_example2() {
    let msg = format!("{:?}", example2("hello".to_string()).unwrap_err());
    assert_eq!(
        msg,
        r"failed with arg hello

Caused by:
    example2: hello"
    )
}

// 'pre' is needed because s is moved into the function
// so the error message needs to be formatted before running the function
#[cu::error_ctx(pre, format("failed with arg {s}"))]
fn example2(s: String) -> cu::Result<()> {
    cu::bail!("example2: {s}")
}

#[tokio::test]
async fn test_example3_err() {
    let msg = format!("{:?}", example3(4).await.unwrap_err());
    assert_eq!(
        msg,
        r"async failed with arg 4

Caused by:
    Condition failed: `value > 4` (4 vs 4)"
    )
}

#[tokio::test]
async fn test_example3_ok() {
    assert!(example3(42).await.is_ok())
}

// question mark works as expected (context is added at return time)
#[cu::error_ctx("async failed with arg {}", s)]
async fn example3(s: u32) -> cu::Result<()> {
    let value = returns_ok(s)?;
    cu::ensure!(value > 4);
    Ok(())
}

#[tokio::test]
async fn test_example4_err() {
    let msg = format!("{:?}", example4("".to_string()).await.unwrap_err());
    assert_eq!(
        msg,
        r"async failed with arg 

Caused by:
    Condition failed: `!value.is_empty()`"
    )
}

#[tokio::test]
async fn test_example4_ok() {
    assert!(example4("hello".to_string()).await.is_ok())
}

// question mark works as expected (context is added at return time)
#[cu::error_ctx(pre, format("async failed with arg {s}"))]
async fn example4(s: String) -> cu::Result<()> {
    let value = returns_ok(s)?;
    cu::ensure!(!value.is_empty());
    Ok(())
}

#[test]
fn test_example5() {
    let msg = format!("{:?}", Foo(7).example5().unwrap_err());
    assert_eq!(
        msg,
        r"Foo failed with arg 7

Caused by:
    example5"
    )
}

// associated functions also work
struct Foo(u32);
impl Foo {
    #[cu::error_ctx("Foo failed with arg {}", self.0)]
    fn example5(&self) -> cu::Result<()> {
        cu::bail!("example5")
    }
}

fn returns_ok<T>(t: T) -> cu::Result<T> {
    Ok(t)
}
