/* Local shim for talloc 2.5.0: bionic API 29 has no memset_explicit().
 * Same as talloc's lib/replace/replace.c rep_memset_explicit (memset plus a compiler barrier). */
#include <string.h>
#include <stddef.h>
void *rep_memset_explicit(void *block, int c, size_t size)
{
	void *ptr = memset(block, c, size);
	__asm__ volatile("" : : "g"(block) : "memory");
	return ptr;
}
