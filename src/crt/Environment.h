#pragma once

#include "IoBuffer.h"
#include "Memory.h"

#define TOTAL_BITS  0x1000
#define MEMORY_SIZE (TOTAL_BITS / sizeof(Cell))

typedef struct _Environment {
    IoBuffer io;
    Memory memory;
} Environment;


int
EnvironmentInit(
    Environment *Env,
    IoBufferConfig *Io,
    Cell *RawMemory,
    uint64_t RawMemorySize
    );
