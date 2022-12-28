# Cauchemar

Cauchemar is a stack-based programming language inspired by [FORTH] but more arcane.

[FORTH]: https://en.wikipedia.org/wiki/Forth_(programming_language)

- Emulates the look and feel of a programming language from the 60s to early 70s.
- Lacks variables and registers.
- Single global stack which stores 32-bit integers, booleans, and string.
- No side-effects, can only print values to a terminal.
- No Read-Eval-Print-Loop.

```cauchemar
PROGRAM:
  "Hello, world!" PRINT       /* Display "Hello, world!"                */
  16 32 + 4 2 * /             /* Calculate (16 + 32) / (4 * 2)          */
  DUP PRINT                   /* Print the result                       */
  DUP 6 EQUALS ASSERT         /* Validate the result                    */
  PLUS-FORTY-TWO              /* Call routine "PLUS-FORTY-TWO"          */
  
  DUP 50 GREATER-THAN         /* Check if the result is greater than 50 */
  IF   "This is wrong" PRINT
  ELSE "This is right" PRINT
  THEN
  
  DO 1 -                      /* Count down to 0                        */
     DUP PRINT 
     DUP 0 GREATER-THAN 
  WHILE
  
  DROP                        /* Reject the top of the stack            */
    
  
PLUS-FORTY-TWO:
  42 +                        /* Add 42 to the top of the stack         */
```

## Planned features

- Registers to store values outside of the stack
- Error backtrace
- Human-readable parser errors
- Terminal input
- More standard routines

## Frequently Asked Questions (F.A.Q)

### Why did you make this?

This programming language came from my dreams and it left a mark on me.

I thought it would be funny to make a real.

### What does the name mean?

"Cauchemar" is the French word for "Nightmare" which is the origin of the
programming language.

### Can I use this on production?

No, that's silly.
