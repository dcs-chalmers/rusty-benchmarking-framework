/// Benchmark statistics structure that stores
/// multiple statistics of multiple benchmark runs.
pub struct BenchmarkStats<'a> {
    /// Column holding samples of a statistic
    stats_columns: Vec<StatColumn<'a>>,
    /// Placeholder for empty values
    empty_placeholder: &'a str,
}

/// Statistic column with a name and set of samples.
struct StatColumn<'a> {
    /// Statistic name
    name: &'a str,
    /// Samples of each iteration
    samples: Vec<Option<String>>,
}

impl<'a> BenchmarkStats<'a> {
    /// Create a new benchmark statistics structure that stores
    /// multiple statistics of multiple benchmark runs.
    /// Replaces unassigned statistic sample values with
    /// the `empty_placeholder` when displaying the statistics.
    pub fn new(empty_placeholder: &'a str) -> Self {
        Self {
            stats_columns: Vec::new(),
            empty_placeholder,
        }
    }

    /// Insert sample value `sample` into the statistic column `stat_name`.
    /// `sample` may not have a value and then `None` can be used instead.
    pub fn insert<V>(&mut self, stat_name: &'a str, sample: Option<V>)
    where
        V: ToString,
    {
        // convert sample to an optional string for storing in the statistics structure
        let sample = sample.map(|value| value.to_string());
        // find the statistic column to insert the sample into
        for stat_column in self.stats_columns.iter_mut() {
            if stat_column.name == stat_name {
                // statistic column exists, insert new sample
                stat_column.samples.push(sample);
                return;
            }
        }
        // statistic column have not been created yet
        // create a new column and insert new sample
        let new_column = StatColumn {
            name: stat_name,
            samples: vec![sample],
        };
        self.stats_columns.push(new_column);
    }

    /// Convert benchmark statistics into csv format returned as a string.
    pub fn to_csv(&self) -> String {
        self.stats_grid()
            .into_iter()
            .map(|row| row.join(","))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Convert benchmark statistics into a pretty print table returned as a string.
    pub fn to_table_string(&self) -> String {
        // find the maximum length of each statistic column
        let column_max_length = self.compute_columns_maximum_length();
        // generate table string row-by-row column-by-column
        let spacing = 4;
        self.stats_grid()
            .into_iter()
            .map(|row| {
                // generate row
                row
                    .into_iter()
                    .enumerate()
                    .map(|(column, cell)| {
                        // ensure that each column of a statistic has the same width
                        let width = column_max_length[column] + spacing;
                        format!("{cell:<width$}")
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            // join each row by a new line
            .join("\n")
    }

    /// Convert benchmark statistics into a grid of strings
    /// where the first row contains the statistic column names
    /// and the following rows contains the statistics of each iteration.
    fn stats_grid(&self) -> Vec<Vec<&str>> {
        // get the number of samples of statistics,
        // ensuring each statistic column have the same number of samples
        let sample_count = self.stat_sample_count()
            .expect("You need to insert the same stats after every benchmark run.");
        // use the statistic column names as the header of the table
        let header: Vec<_> = self.stats_columns
            .iter()
            .map(|stat| stat.name)
            .collect();
        // get the samples row-by-row column-by-column
        let mut sample_rows = (0..sample_count).map(|iteration| {
            // get a row by iterating over all columns at that row
            self.stats_columns.iter().map(|stat| {
                stat.samples[iteration]
                    .as_ref()
                    // use the placeholder value if the value string is not given
                    .map_or(self.empty_placeholder, |value| value.as_str())
            })
            .collect()
        }).collect();
        // construct grid of strings with header as the first row
        let mut grid = vec![header];
        grid.append(&mut sample_rows);
        grid
    }
 
    /// Get the number of samples of statistics.
    /// If not all statistic columns have the same number of samples,
    /// then some samples are missing which is an error.
    /// This is indicated by returning `None`.
    fn stat_sample_count(&self) -> Option<usize> {
        let count = self.stats_columns
            .first()
            .map(|stat| stat.samples.len())
            .unwrap_or(0);
        if self.stats_columns.iter().all(|stat| stat.samples.len() == count) {
            Some(count)
        } else {
            None
        }
    }

    /// Find the maximum length of each statistic column.
    fn compute_columns_maximum_length(&self) -> Vec<usize> {
        // maximum length is initially set to the colum header length
        let mut column_max_length: Vec<_> = self.stats_columns
            .iter()
            .map(|stat| stat.name.chars().count())
            .collect();
        let placeholder_len = self.empty_placeholder.chars().count();
        for (column, stat) in self.stats_columns.iter().enumerate() {
            // find the maximum length of the sample strings of the current column
            let sample_max_len = stat.samples
                .iter()
                .map(|sample| {
                    sample
                        .as_ref()
                        .map(|value_str| value_str.chars().count())
                        // if sample does not have a value, the placeholder's length is used
                        .unwrap_or(placeholder_len)
                })
                .max()
                .unwrap_or(0);
            // compare the maximum sample string length to the column header length
            column_max_length[column] = column_max_length[column].max(sample_max_len);
        }
        // return the found maximum lengths
        column_max_length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_table_string() {
        let stats = sample_stats();
        println!("{}", stats.to_table_string());
    }

    #[test]
    #[ignore]
    fn test_csv() {
        let stats = sample_stats();
        println!("{}", stats.to_csv());
    }

    fn sample_stats() -> BenchmarkStats<'static> {
        let mut stats = BenchmarkStats::new("*");
        for _ in 0..10 {
            stats.insert("Throughput", Some(323.322));
            stats.insert("Find", Some(2121));
            stats.insert("Inserts", Some(3232));
            stats.insert("Removes", Some(32));
            stats.insert("Data Structure", Some("Some Type"));
            stats.insert("Thread Count", Some(4));
        }
        stats.insert("Throughput", Option::<String>::None);
        stats.insert("Find", Option::<String>::None);
        stats.insert("Inserts", Option::<String>::None);
        stats.insert("Removes", Option::<String>::None);
        stats.insert("Data Structure", Some("Some Type"));
        stats.insert("Thread Count", Some(4));
        stats
    }
}
