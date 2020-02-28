#include <stdio.h>
#include <string.h>

#include "IoBuffer.h"
#include "Memory.h"
#include "Environment.h"
#include "ProgramCtx.h"

int
main(
    void
    )
{
    int status;
    Cell rawMemory[MEMORY_SIZE];
    IoBufferConfig io;
    Environment env;

    status = 0;

    memset(&rawMemory[0], 0, sizeof(rawMemory));

    io.GetByte = (GetByteFn) getc;
    io.GetContext = stdin;
    io.PutByte = (PutByteFn) putc;
    io.PutContext = stdout;

    CHECK(status = EnvironmentInit(&env, &io, &rawMemory[0], MEMORY_SIZE));

    Program(&env);
    hexdump("Tape", &rawMemory[0], sizeof(rawMemory));

Bail:
    return (status != 0);
}
