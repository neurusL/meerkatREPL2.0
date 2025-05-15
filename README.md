# Meerkat 2.0 a distributed reactive programming language 

## Grammar
```
<params> ::= epsilon | <ident> <params_follow>
<params_follow> ::= epsilon | , <ident> <params_follow>
<unop> ::= ! | - 
<binop> ::= + | - | * | / | == | < | > | && | ||
<expr> ::= <ident> | <const> 
| <unop> <expr> | <expr> <binop> <expr>
| if <expr> then <expr> else <expr>
| fn ( <params> ) => <expr>
| action { }
```

## example of uses 
``` Meerkat

```