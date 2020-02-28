#include "ProgramCtx.h"


/*
void
debug(
    Environment *Env
    )
{
    Env->memory.head = 0;
    Env->memory.cache.head = 0xffffffffffffffffUL;

    for (;;)
    {
        SET();
        SHIFTR(2);
    }
Bail:
    (void) 0;
    __builtin_trap();
}
*/

void
Program(
    Environment *Env
    )
{
#include "Program.h"

Bail:
    (void) 0;
//    __builtin_trap();
}
