
```
// Variables:
// PRIMARY_LANGUAGE   — user’s primary language
// SECONDARY_LANGUAGE — second language (most common translation from PRIMARY_LANGUAGE)
// LAST_LANGUAGE      — last selected target language (or null)
// SRC                — language of the source text

function chooseTargetLanguage(SRC, PRIMARY_LANGUAGE, SECONDARY_LANGUAGE, LAST_LANGUAGE):
    // 1. If the source isn’t the primary language, translate into the primary language
    if SRC ≠ PRIMARY_LANGUAGE:
        return PRIMARY_LANGUAGE

    // 2. If the source is the primary language and there’s a meaningful last choice, use it
    if LAST_LANGUAGE ≠ null AND LAST_LANGUAGE ≠ PRIMARY_LANGUAGE:
        return LAST_LANGUAGE

    // 3. Otherwise, fall back to the secondary language
    return SECONDARY_LANGUAGE

```
