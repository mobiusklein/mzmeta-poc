This is a proof-of-concept for embedding sample or experimental metadata in mzML or
a compatible data model.

## Theorized usage

The idea behind this program was to see what kind of information might be portable between
an SDRF file and an mzML file.

```bash
msconvert path/to/RAW | mzmeta path/to/sdrf > path/to/mzML
```
