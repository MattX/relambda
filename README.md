# Relambda

## An [Unlambda](http://www.madore.org/~david/programs/unlambda/) interpreter in Rust

Relambda is a bytecode-compiling [Unlambda](http://www.madore.org/~david/programs/unlambda/) interpreter in Rust.

```
$ relambda
>> `r`.!`.d`.l`.r`.o`.w`. `.,`.o`.l`.l`.e`.Hi
Hello, world!
```

You can find more sample programs at the
[comprehensive Unlambda archive network](ftp://ftp.madore.org/pub/madore/unlambda/CUAN/),
and a tutorial and language spec on the [Unlambda homepage](http://www.madore.org/~david/programs/unlambda/).

## Language support

Relambda supports Unlambda 2.0. It supports arbitrary Unicode characters after `.`, where the standard supports
ASCII. This means that you cannot print out raw bytes in the 127-255 range. Code is case insensitive, except for `.`
characters. Comments are supported.

## Design notes

Unlambda is compiled to a 6-instruction bytecode. Due to its very dynamic nature, most of the work is dynamically
dispatched by the `Invoke` opcode.

## Testing

Some integ tests are included. I think I've tested all programs in the
CUAN (`ftp://ftp.madore.org/pub/madore/unlambda/CUAN/`). The `test-runner` script will run
[the suite of tests at `ftp://ftp.madore.org/pub/madore/unlambda/tests/unlambda-tests`](https://bit.ly/32lcbMA)
(which you need to download if you want to run them—I am not including them here as this file has no copyright
information.) 

## Speed

Very informal testing suggests that this interpreter is quite a bit faster than the C-refcount interpreter included in
the official CPAN distribution. 
