fn decide(x: i32, y: i32) -> i32 {
    if x > 0 {
        if y > 0 {
            x + y
        } else {
            x - y
        }
    } else if x == 0 {
        match y {
            0 => 0,
            1 => 1,
            _ => y * 2,
        }
    } else {
        x * y
    }
}
