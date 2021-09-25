# ftrace

> woah, it's strace, but for functions

Trace the functions reciding inside a compiled binary.

## Usage

Take this simple C file,

```C
#include <stdio.h>

int fact(int x) {
    if (x < 1) {
        return 1;
    }
    return x * fact(x - 1);
}

int main() {
    printf("%d", fact(5));
}
```

Compile it and run `ftrace`

```
$ gcc fact.c -o fact
$ cargo run ./fact
| _start(140635318833104)
| | __libc_csu_init(1, 140736041307608, 140736041307624)
| | | frame_dummy()
| | | | register_tm_clones()
| | | | 0
| | | 0
| | | main()
| | | | fact(10)
| | | | | fact(9)
| | | | | | fact(8)
| | | | | | | fact(7)
| | | | | | | | fact(6)
| | | | | | | | | fact(5)
| | | | | | | | | | fact(4)
| | | | | | | | | | | fact(3)
| | | | | | | | | | | | fact(2)
| | | | | | | | | | | | | fact(1)
| | | | | | | | | | | | | | fact(0)
| | | | | | | | | | | | | | 1
| | | | | | | | | | | | | 1
| | | | | | | | | | | | 2
| | | | | | | | | | | 6
| | | | | | | | | | 24
| | | | | | | | | 120
| | | | | | | | 720
| | | | | | | 5040
| | | | | | 40320
| | | | | 362880
| | | | 3628800
| | | 0
| | | __do_global_dtors_aux()
| | | | deregister_tm_clones()
| | | | 4210736
| | | 4210736
```

## stuff it can't do (yet)
- use types from DWARF info
