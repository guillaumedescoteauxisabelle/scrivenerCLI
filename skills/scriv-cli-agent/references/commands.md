# scriv Command Reference (Agent Quick Sheet)

## Project
- `scriv --project <path> project info`
- `scriv --project <path> project validate --strict`
- `scriv --project <path> project doctor --check`

## Tree
- `scriv --project <path> tree ls --recursive`
- `scriv --project <path> tree mkdir --path "Draft/Act II/New Chapter"`
- `scriv --project <path> tree mkdoc --path "Draft/Act II/New Chapter/Scene 1"`
- `scriv --project <path> tree mv --from "..." --to "..."`
- `scriv --project <path> tree reorder --path "..." --before "..."`
- `scriv --project <path> tree rm --path "..." --force`

## Documents
- `scriv --project <path> doc cat --id <uuid>`
- `scriv --project <path> doc write --id <uuid> --stdin`
- `scriv --project <path> doc append --id <uuid> --stdin`
- `scriv --project <path> doc prepend --id <uuid> --stdin`
- `scriv --project <path> doc edit --id <uuid> --set-title "..."`

## Metadata
- `scriv --project <path> meta notes get --id <uuid>`
- `scriv --project <path> meta notes set --id <uuid> --stdin`
- `scriv --project <path> meta synopsis get --id <uuid>`
- `scriv --project <path> meta synopsis set --id <uuid> --text "..."`

## Sync + Conflict
- `scriv --project <path> sync pull`
- `scriv --project <path> sync push`
- `scriv --project <path> sync status`
- `scriv --project <path> conflict status`
- `scriv --project <path> conflict resolve --id <uuid> --use mirror`

## Compile
- `scriv --project <path> compile run --format md --output /tmp/out.md`
- `scriv --project <path> compile run --format txt --output /tmp/out.txt`
