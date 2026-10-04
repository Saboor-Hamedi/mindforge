# MindForge — Fix Autocomplete Enter Conflict

There is now one specific regression in the working LSP autocomplete system.

When I type something such as:

```text
h1
```

the correct autocomplete suggestion becomes selected.

However, when I press **Enter**, instead of accepting the selected LSP completion, MindForge triggers another application action in the background — apparently the Wikilink/note-opening handler.

The result is:

1. Autocomplete suggestion is selected.
2. I press Enter.
3. The completion is NOT accepted correctly.
4. Another note opens unexpectedly.
5. When I return, autocomplete is no longer behaving correctly.

## Required behavior

When the LSP completion popup is visible and an item is selected:

```text
Enter
```

must be **consumed by the completion system first**.

It must:

```text
Enter
  ↓
accept selected completion
  ↓
insert completion
  ↓
close completion popup
```

and **must NOT propagate to**:

- Wikilink handling
- note opening
- global Enter handlers
- tabs
- command system
- any other application shortcut

Conceptually:

```text
Keyboard Enter
      ↓
Completion popup active?
      │
     YES
      ↓
Accept completion
      ↓
CONSUME EVENT
      X
No further global handlers
```

Only when completion is **not** active should Enter continue through the normal MindForge input pipeline.

### Important

Do not rewrite the LSP or completion system.

Find where Enter is currently routed and fix the event-consumption/priority issue at the correct input-routing layer.

Also verify that after accepting:

```text
h1 + Enter
```

the completion popup closes normally and autocomplete continues working for subsequent typing.

Test specifically that the Wikilink/note-opening handler does not receive the same Enter event.

Keep the fix small and focused. Do not modify unrelated editor functionality.
