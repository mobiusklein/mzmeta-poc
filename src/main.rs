//! Read an SDRF file, and an mzML file expected to be on `STDIN`,
//! modifying it en route to include a sample list with metadata
//! drawn from the SDRF file

use std::{borrow::Cow, collections::HashMap, env, io, path, sync::Arc};

use csv;
use log::info;
use mzdata::{
    curie,
    io::{MzMLReader, MzMLWriter, StreamingSpectrumIterator},
    meta::Sample,
    params::{Param, ParamValue, Value},
    prelude::*,
};

/// Describe a column class tag
#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum SDRFClass {
    #[default]
    Innate,
    Characteristic,
    Comment,
    Factor,
}


/// Represent a row value for a single column in an SDRF table
#[derive(Default, Clone)]
struct SDRFField {
    name: Arc<String>,
    field_class: SDRFClass,
    value: Value,
}

impl SDRFField {

    /// Convert this column into an mzML-compatible parameter, either as a cvParam or userParam
    fn as_param(&self) -> Param {
        let name = self.name();

        // Check to see if the column is one we have a clear controlled vocabulary mapping for
        let curie_of = match name {
            "organism part" => Some((curie!(EFO:0000635), name, self.value.clone())),
            "organism" => Some((curie!(OBI:0100026), name, self.value.clone())),
            "developmental stage" => Some((curie!(EFO:0000399), name, self.value.clone())),
            "ancestry category" => Some((curie!(HANCESTRO:0004), name, self.value.clone())),
            "cell type" => Some((curie!(EFO:0000324), name, self.value.clone())),
            "material type" => Some((curie!(BFO:0000040), name, self.value.clone())),
            "age" => Some((curie!(EFO:0000246), name, self.value.clone())),
            "disease" => Some((curie!(EFO:0000408), name, self.value.clone())),

            "time" => Some((curie!(EFO:0000721), name, self.value.clone())),
            "technology type" => Some((curie!(EFO:0005521), name, self.value.clone())),

            "biological replicate" => Some((curie!(EFO:0002091), name, self.value.clone())),
            "technical replicate" => Some((curie!(MS:1001808), name, self.value.clone())),
            "fraction identifier" => Some((curie!(MS:1000858), name, self.value.clone())),

            "file uri" => Some((curie!(PRIDE:0000577), name, self.value.clone())),

            // TMT labels (and probably other isobaric labels)
            // TODO: The MS controlled vocabulary has specific terms for these labels, but the PRIDE CV seems to
            // have its own terms for them, sometimes in multiples? Which CV would it make sense to use here?
            "label" => match self.value.as_str().as_ref() {
                "TMT126" => Some((curie!(MS:1002616), "TMT reagent 126", Value::Empty)),
                "TMT127" => Some((curie!(MS:1002617), "TMT reagent 127", Value::Empty)),
                "TMT128" => Some((curie!(MS:1002618), "TMT reagent 128", Value::Empty)),
                "TMT129" => Some((curie!(MS:1002619), "TMT reagent 129", Value::Empty)),
                "TMT130" => Some((curie!(MS:1002620), "TMT reagent 130", Value::Empty)),
                "TMT131" => Some((curie!(MS:1002621), "TMT reagent 131", Value::Empty)),
                "TMT127N" => Some((curie!(MS:1002763), "TMT reagent 127N", Value::Empty)),
                "TMT127C" => Some((curie!(MS:1002764), "TMT reagent 127C", Value::Empty)),
                "TMT128N" => Some((curie!(MS:1002765), "TMT reagent 128N", Value::Empty)),
                "TMT128C" => Some((curie!(MS:1002766), "TMT reagent 128C", Value::Empty)),
                "TMT129N" => Some((curie!(MS:1002767), "TMT reagent 129N", Value::Empty)),
                "TMT129C" => Some((curie!(MS:1002768), "TMT reagent 129C", Value::Empty)),
                "TMT130N" => Some((curie!(MS:1002769), "TMT reagent 130N", Value::Empty)),
                "TMT130C" => Some((curie!(MS:1002770), "TMT reagent 130C", Value::Empty)),
                _ => None,
            },
            _ => None,
        };
        if let Some((curie_of, name, value)) = curie_of {
            curie_of
                .controlled_vocabulary
                .param_val(curie_of.accession, name, value)
        } else {
            Param::new_key_value(self.name.to_string(), self.value.clone())
        }
    }

    /// Extract the name of the column, independent of its column class
    fn name(&self) -> &str {
        match self.field_class {
            SDRFClass::Innate | SDRFClass::Factor => self.name.as_str(),
            _ => self
                .name
                .split_once('[')
                .unwrap()
                .1
                .rsplit_once(']')
                .unwrap()
                .0
                .trim(),
        }
    }
}

#[derive(Default, Clone)]
struct SDRFSample {
    name: String,
    fields: Vec<SDRFField>,
    characteristics: Vec<SDRFField>,
    comments: Vec<SDRFField>,
    factors: Vec<SDRFField>,
}

impl SDRFSample {
    fn data_file(&self) -> Option<Cow<'_, str>> {
        self.comments
            .iter()
            .find(|f| f.name() == "data file")
            .map(|f| f.value.as_str())
    }

    fn as_sample(&self) -> Sample {
        let mut params = Vec::new();
        for field in self
            .fields
            .iter()
            .chain(self.characteristics.iter())
            .chain(self.comments.iter())
            .chain(self.factors.iter())
        {
            match field.name() {
                "data file" | "instrument" => {}
                _ => params.push(field.as_param()),
            }
        }
        Sample::new(
            self.name.replace(" ", "_").to_lowercase(),
            Some(self.name.to_string()),
            params,
        )
    }
}

/// Actually read the SDRF file into row-level [`SDRFSample`].
///
/// Makes no effort to aggregate replicates
fn read_sdrf(sdrf_path: &path::Path) -> io::Result<Vec<SDRFSample>> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(true)
        .from_path(sdrf_path)?;

    // Read and normalize the column names. These will be re-used over rows of [`SDRFField`].
    let headers: Vec<_> = reader
        .headers()?
        .iter()
        .map(|s| Arc::new(s.to_string().replace(" ]", "]")))
        .collect();

    // Parse the rows into [`SDRFSample`] instances
    let mut samples = Vec::new();
    for (i, row) in reader.records().enumerate() {
        match row {
            Ok(row) => {
                let mut sample = SDRFSample::default();
                for (name, val) in headers.iter().zip(row.iter()) {
                    match name.as_str() {
                        "source name" => sample.name = val.into(),
                        x if x.starts_with("characteristics[")
                            || x.starts_with("characteristic[") =>
                        {
                            let f = SDRFField {
                                name: Arc::clone(name),
                                field_class: SDRFClass::Characteristic,
                                value: val.parse().unwrap(),
                            };
                            sample.characteristics.push(f);
                        }
                        x if x.starts_with("comment[") => {
                            let f = SDRFField {
                                name: name.clone(),
                                field_class: SDRFClass::Comment,
                                value: val.parse().unwrap(),
                            };
                            sample.comments.push(f);
                        }
                        x if x.starts_with("factor value[") => {
                            let f = SDRFField {
                                name: Arc::clone(name),
                                field_class: SDRFClass::Factor,
                                value: val.parse().unwrap(),
                            };
                            sample.factors.push(f);
                        }
                        _ => {
                            let f = SDRFField {
                                name: Arc::clone(name),
                                field_class: SDRFClass::Innate,
                                value: val.parse().unwrap(),
                            };
                            sample.fields.push(f);
                        }
                    }
                }
                samples.push(sample);
            }
            Err(e) => {
                eprintln!("Failed to parse SDRF line {i}: {e}");
                return Err(e.into());
            }
        }
    }

    Ok(samples)
}

/// Re-arrange the samples into groups organized by the "comment[data file]" field
fn organize_by_data_file(sdrf_samples: Vec<SDRFSample>) -> HashMap<String, Vec<SDRFSample>> {
    let mut index: HashMap<String, Vec<SDRFSample>> = HashMap::new();
    for s in sdrf_samples {
        index
            .entry(s.data_file().unwrap().to_string())
            .or_default()
            .push(s);
    }
    index
}

/// Consume the mzML stream from STDIN and write it through to STDOUT
fn write_passthrough<R: io::Read>(reader: MzMLReader<R>) -> io::Result<()> {
    let reader_iter = StreamingSpectrumIterator::new(reader);
    let write_stream = io::stdout().lock();
    let mut writer = MzMLWriter::new(write_stream);
    writer.copy_metadata_from(&reader_iter);
    let mut n_spectra_so_far = 0;
    for (i, group) in reader_iter.into_groups().enumerate() {
        if i % 500 == 0 && i > 0 {
            info!("Writing group {i}, {n_spectra_so_far} spectra written");
        }
        n_spectra_so_far += group.total_spectra();
        writer.write_group_owned(group)?;
    }
    info!("Wrote {n_spectra_so_far} spectra");
    Ok(())
}

/// Patch the metadata with the samples that correspond to this data file, as given by the "first"
/// source file in this mzML file
fn update_sample_list<R: io::Read>(reader: &mut MzMLReader<R>, samples: &[SDRFSample]) {
    let samples_of = reader.samples_mut();
    samples_of.clear();
    samples_of.extend(samples.iter().map(|s| s.as_sample()));
    info!("Updated sample list metadata");
}

fn main() -> io::Result<()> {
    pretty_env_logger::init_timed();
    let sdrf_path = path::PathBuf::from(env::args().skip(1).next().unwrap());
    let samples = read_sdrf(&sdrf_path)?;
    let samples_by_data_file = organize_by_data_file(samples);

    let mut reader = MzMLReader::new(io::stdin());

    let source_file = reader.file_description().source_files.first().unwrap();
    log::info!("Extracting samples associated with {}", source_file.name);
    let samples = samples_by_data_file.get(&source_file.name).unwrap();
    log::info!("Found {} samples", samples.len());

    update_sample_list(&mut reader, &samples);

    write_passthrough(reader)?;
    Ok(())
}
