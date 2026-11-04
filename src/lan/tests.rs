use super::*;

#[test]
fn turning_on_writes_any_and_turning_off_restores_the_default() {
    assert_eq!(
        change_params(true),
        json!({"changes": [{"key": "web.listen", "value": "0.0.0.0"}]})
    );
    assert_eq!(
        change_params(false),
        json!({"changes": [{"key": "web.listen", "unset": true}]})
    );
}

#[test]
fn only_a_hello_that_says_so_has_a_password() {
    assert!(password_set(&json!({"password_set": true})));
    assert!(!password_set(&json!({"password_set": false})));
    // 老核心不说的当没设：先去设密码，不冒险打开。
    assert!(!password_set(&json!({"language": "zh"})));
}
