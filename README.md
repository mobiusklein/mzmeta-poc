This is a proof-of-concept for embedding sample or experimental metadata in mzML or
a compatible data model.

## Theorized usage

The idea behind this program was to see what kind of information might be portable between
an SDRF file and an mzML file. The SDRF file includes a `comment[data file]` column which
tells you which raw file (or mzML file) a row describes. We can build the `sampleList`
for an mzML file from the SDRF rows that correspond to its source RAW file.

```bash
msconvert path/to/RAW | mzmeta path/to/sdrf > path/to/mzML
```

## Example

[PXD017710](https://www.ebi.ac.uk/pride/archive/projects/PXD017710) has been annotated with an SDRF file, describing
a TMT multiplexing experiment with multiple samples per MS run. If `mzmeta` were run on `20200219_KKL_SARS_CoV2_pool1_F1.raw`
being converted to mzML, it would pull in details describing each of the samples based upon the rows listed for that data file
in the SDRF file.

```xml
    <sampleList count="9">
      <sample id="sample_1" name="Sample 1">
        <cvParam accession="BFO:0000040" cvRef="BFO" name="material type" value="cell"/>
        <userParam type="xsd:string" name="assay name" value="run 1"/>
        <cvParam accession="EFO:0005521" cvRef="EFO" name="technology type" value="proteomic profiling by mass spectrometry"/>
        <cvParam accession="OBI:0100026" cvRef="OBI" name="organism" value="Homo sapiens"/>
        <cvParam accession="EFO:0000635" cvRef="EFO" name="organism part" value="colon"/>
        <userParam type="xsd:string" name="characteristics[sex]" value="male"/>
        <cvParam accession="EFO:0000246" cvRef="EFO" name="age" value="72"/>
        <cvParam accession="EFO:0000399" cvRef="EFO" name="developmental stage" value="adult"/>
        <cvParam accession="HANCESTRO:0000004" cvRef="HANCESTRO" name="ancestry category" value="caucasian"/>
        <cvParam accession="EFO:0000324" cvRef="EFO" name="cell type" value="not available"/>
        <cvParam accession="EFO:0000408" cvRef="EFO" name="disease" value="colon cancer"/>
        <userParam type="xsd:string" name="characteristics[cell line]" value="CaCo-2"/>
        <userParam type="xsd:string" name="characteristics[infect]" value="bridge mixed pool"/>
        <cvParam accession="EFO:0000721" cvRef="EFO" name="time" value="none"/>
        <cvParam accession="EFO:0002091" cvRef="EFO" name="biological replicate" value="1"/>
        <cvParam accession="MS:1002621" cvRef="MS" name="TMT reagent 131"/>
        <cvParam accession="PRIDE:0000577" cvRef="PRIDE" name="file uri" value="https://ftp.pride.ebi.ac.uk/pride/data/archive/2020/03/PXD017710/20200219_KKL_SARS_CoV2_pool1_F1.raw"/>
        <cvParam accession="MS:1000858" cvRef="MS" name="fraction identifier" value="1"/>
        <cvParam accession="MS:1001808" cvRef="MS" name="technical replicate" value="1"/>
        <userParam type="xsd:string" name="comment[modification parameters]" value="NT=TMT6plex;PP=Any N-term;AC=UNIMOD:737;MT=fixed"/>
        <userParam type="xsd:string" name="comment[modification parameters]" value="NT=Carbamidomethyl;TA=C;AC=UNIMOD:4;MT=fixed"/>
        <userParam type="xsd:string" name="comment[modification parameters]" value="NT=Oxidation;TA=M;AC=UNIMOD:35;MT=variable"/>
        <userParam type="xsd:string" name="comment[modification parameters]" value="NT=13C6-15N4;TA=R;AC=UNIMOD:267;MT=variable"/>
        <userParam type="xsd:string" name="comment[cleavage agent details]" value="NT=Trypsin"/>
        <userParam type="xsd:string" name="comment[cleavage agent details]" value="NT=Lys-C"/>
        <userParam type="xsd:string" name="comment[fragment mass tolerance]" value="not available"/>
        <userParam type="xsd:string" name="comment[precursor mass tolerance]" value="not available"/>
        <userParam type="xsd:string" name="factor value[infect]" value="bridge mixed pool"/>
        <userParam type="xsd:string" name="factor value[time]" value="none"/>
      </sample>
```

In some cases, I could map the columns to controlled vocabulary terms. In others, I chose
to just roundtrip all the SDRF field details as-is lacking a clear non-lossy mechanism
for encoding that information.