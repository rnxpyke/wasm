(module
    (func $add (export "add") (param i32) (param i32) (result i32)
        (local.get 0)
        (local.get 1)
        i32.add
    )
    (func $s
        (i32.const 1) (i32.const 2)
        (call $add)
        drop
    )
    (start $s)
)