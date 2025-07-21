# Meerkat 2.0 a distributed reactive programming language 
## Introduction

## Grammar
```rust
<params> ::= epsilon | <ident> <params_follow>
<params_follow> ::= epsilon | , <ident> <params_follow>
<unop> ::= ! | - 
<binop> ::= + | - | * | / | == | < | > | && | ||
<expr> ::= <ident> | <const> 
| <unop> <expr> | <expr> <binop> <expr>
| if <expr> then <expr> else <expr>
| fn <params> => <expr>
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

## Example of common uses 
Meerkat source code:
``` 
var x = 1;
var y = 2;
var foo = fn id => id

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

## Semantics
Meerkat2.0 extends lambda calculus with reactive assignable names for demostration of idea, (an inproper analogy is ```ref``` cells in OCaml). More fundamentally, Meerkat2.0 shares the same idea of extending system PCF with assignables results in Modernized Algol(MA), with some significant difference in dynamics.

TODO:
there are several todos to make Meerkat2.0 more developer friendly and practical:
- think about how to integrate Meerkat core into modern languages(resemble nowadays frontend languages like TypeScript), or implement equivalent functionalities, including
    - general program structure: TODO: should it only be a vector of declaration?
    - scoping and lambda calculus variable
    - non-reactive ref cells
    - data structures (I believe integrate Meerkat into other well-developed languages might be a save of time)
    - ...
- support API to databases, instead of REPL maintaining data locally

### Statics
Similar to Lambda Calculus and Modern Algol, actions are encapsulated in expression as suspended computation for following reasons:
- modal separation (TODO)
- prevent divergent behavior of actions, by triggering the computation only when client asked to do the action

### Dynamics
The reactive part of Meerkat is defined by 
- ```var```'s, the reactive name/assignable can be updated, and they cannot depend on other reactive names (closed expressions)
- ```def```'s, the reactive name/assignable depending on others, and they are automatically updated when their predecessors have update

## Research Problem
TODO: more to fill here
### Possibilities: 
https://docs.google.com/document/d/182irBVCUuOa2P_xdoZrRFmhh4necxawbaSv5MKFuA0g/edit?usp=sharing

### Historiographer Protocols v1: 
https://docs.google.com/document/d/1oZFCzLQCPoAA_3Lc2OX89AGdp95dzSmu7rS9FGetEpA/edit?usp=sharing

### Historiographer Protocols v2: 
dhttps://docs.google.com/document/d/1VmBqQindHkSDmaNuVQ_vBsn-lRJ-Gg94hZ_G0UnRzrY/edit?usp=sharing


## Implementation Detail
(main branch, latest modified Jun 22 2025)
### System Structure Design
MeerkatREPL2.0 has following components:
1. developers and clients
Developers and clients are outsiders of MeerkatREPL, interacting with MeerkatREPL, both of which are similated by ```main``` function communicating with REPL through two ```tokio``` channels:
```rust
pub async fn run(prog: &Prog) -> Result<(), Box<dyn std::error::Error>> {
    let (dev_tx, dev_rx) = mpsc::channel::<CmdMsg>(MPSC_CHANNEL_SIZE);
    let (cli_tx, cli_rx) = mpsc::channel::<CmdMsg>(MPSC_CHANNEL_SIZE);
    ...
}
```

2. managers
Managers act as the heart of MeerkatREPL: on one side connecting to clients and developers who constantly sending updates to reactive names and updates to code base; on the other side managing local ```var``` and ```def``` actors' states. Though ```var``` and ```def``` actors maintain their own values and lock status, manager plays a key role of processing actions into transaction, evaluating a snip of code as non-distributed interpreters do, and initiate 2 Phase Lock for all local and involved remote ```def/var``` actors

3. ```var``` actors
```var``` actors are reactive names(assignables) with no predecessors, and can be re-assigned by clients. They are more interesting than ```def``` actors in terms of lock. Besides ```UpgradeLock``` requested by code update, ```WriteLock``` and ```ReadLock``` are also requested to ```var``` actors by code update from developers, as well as value update from clients. ```var``` actor uses wait-die mechanism to decide when to allow a lock to wait, and when to abort. The symphony of locks mainly happens at ```var``` actors.

4. ```def``` actors
```def``` actors are reactive names(assignables) depending on some other names, but they cannot be explicitly re-assigned by clients. They are more interesting than ```var``` actors in terms of glitch-freedom and consistency. They receive messages of form ```(def's predecessor name := new_value, $$P set$$)```, where $$P$$ is a set of transactions has to be applied when ```def``` actor formally apply ```name := new_value```. ```def``` actors accumulate and wisely search for a batch of messages to meet the requirement, manifested in the system with a good property of causal consistency and glitch freedom.

5. ```service```
In our ambitious design, we allocate managers for ```service```'s dynamically based on their locality, size, usage, etc. Such an optimization might be realized in non-distant future, but for now we did the simplest design:
each service has a unique manager, who managing all the ```def``` and ```var```'s declared by the service, additionally managing channels by which developer and client connected to the service.


### Distributed System Protocol



