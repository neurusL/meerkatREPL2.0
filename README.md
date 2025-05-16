# Meerkat 2.0 a distributed reactive programming language 

## Grammar
```rust
<params> ::= epsilon | <ident> <params_follow>
<params_follow> ::= epsilon | , <ident> <params_follow>
<unop> ::= ! | - 
<binop> ::= + | - | * | / | == | < | > | && | ||
<expr> ::= <ident> | <const> 
| <unop> <expr> | <expr> <binop> <expr>
| if <expr> then <expr> else <expr>
| fn ( <params> ) => <expr>
| action { <assign>* }

<decl> ::=
| var <ident> = <expr>;
| def <ident> = <expr>;
| pub def <ident> = <expr>;
| import <ident>    // import var/defs from other services
<decls> ::= <decl>*

<assign> ::= 
| <ident> = <expr>; // assign expression to reactive var name 
// | do <expr>      // in this version we avoid this, but subject to discussion

<service> ::= service <ident> { <decls> }

<prog> ::= <service>*
<repl_input> ::= do <expr>
```

## Example of uses 
Meerkat source code:
``` 
var x = 1;
var y = 2;
var foo = fn (id) => id

def xy = x * y;
def inc_x_by_1 = action { x = x + 1 };
def dec_x_by_1 = action { x = x - 1 };
def change_foo = action { foo = fn (id) => id + 1}

def dec_cond_x = 
    if x > 5 
    then dec_x_by_1
    else change_foo

```

How client interact:
```
do inc_x_by_1
do inc_x_by_1
do inc_x_by_1
do inc_x_by_1
do dec_cond_x
do inc_x_by_1
do dec_cond_x
exit
```