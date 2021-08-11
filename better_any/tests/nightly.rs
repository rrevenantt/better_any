use better_any::{downcast_any, DowncastExt};
use std::any::Any;
#[test]
fn test() {
    use std::fmt::Debug;
    let a = 5i32;
    let any = &a as &dyn Any;
    let result: &i32 = downcast_any(any).unwrap();
}

//should fail to compile
