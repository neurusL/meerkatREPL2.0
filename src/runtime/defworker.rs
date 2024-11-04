// defworker 
// read lock 

// append following to existing meerkat project:
/*
    var a = 0;
    var b = 0;
    def f = a + b;
    def g = f;
*/
/*
Service Manager:
- create all worker a, b, f OR require RLocks for {a, b} and WLock for {f}
- for code update, processing line l+1 will be blocked by line l (dependency)
- for newly created 
- gather all typing inforation permitted by RLock, info directly from typing env
  maintained by service manager 
- defworker process Write: 
    - establish all dependencies (graph's structure) by subscribption,
    - gain all dependencies' values (current inputs)
*/

// One problem:
// for a defwork, code update vs pending change message (v, P, R), which apply first
// - idea 1 : ignore all pending change messages 
// - idea 2 : wait for all pending change messages to finish
/*
update {
    var c = 1;
    def f' = a * b * c; // if read(f') >= read(f), then batch validity remains 
    def g = f;
}
*/

/*
update {
    var c = 1;
    def f' = a;         // wait for pending messages to finish, then apply code update
    def g = f;          // since now batch validity changes
}
*/



/*
update {
    var c = 1;
    def f' = a * b * c;  
    def g = f;
}
 |concurrently|
update {
    var c = 1;
    def f' = a;         
    def g = f;          
}
*/
// one WLock rejected 


/*
var c = 1;
def f1 = a; 
def f2 = b;
def g = f1 + f2;

dev1 update {
    def f2 = b + c;
}
 |concurrently|
dev2 update {
    def f1 = a + c; 
}
*/
// dev1:: WLock: {f1}; RLock: {b, c, g}
// dev2:: WLock: {f2}; RLock: {a, c, g}



// grant write lock to developer

// process Write Message from developer
// process write lock