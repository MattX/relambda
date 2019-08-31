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

Relambda supports Unlambda 2.0. It supports arbitrary Unicode characters after `.`, where the standard only supports
ASCII. Code is case insensitive, except for `.` characters. Comments are supported.

## Design notes

Unlambda compiles to a small bytecode with 
