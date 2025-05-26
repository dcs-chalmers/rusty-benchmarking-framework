import os
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path

def plot_bfs_benchmarks(youtube_folder, twitter_folder, graph_info=None):
    """
    Plot BFS benchmark data from files in the specified folders.
    
    Args:
        youtube_folder (str): Path to folder containing soc-youtube data files
        twitter_folder (str): Path to folder containing soc-twitter-2010 data files
        graph_info (dict, optional): Dictionary with graph information like:
            {'soc-youtube': {'vertices': 1134890, 'edges': 2987624},
             'soc-twitter-2010': {'vertices': 41652230, 'edges': 1468365182}}
    """
    
    # Default graph information if not provided
    if graph_info is None:
        graph_info = {
            'soc-youtube': {'vertices': 495957, 'edges': 1936748},
            'soc-twitter-2010': {'vertices': 21297772, 'edges': 265025809}
        }
    
    name_translator = {
            "faaaq_rust_optimised" : "FAAAQueue Optimised",
            "faaaq_rust_unoptimised" : "FAAAQueue Unoptimised",
            "faaa_queue_cpp" : "C++ FAAAQueue",
            "lprq_rust_correct" : "Rust LPRQ Optimised",
            "lcrq_rust_correct" : "Rust LCRQ Optimised",
            "lcrq_cpp" : "C++ LCRQ",
            "lprq_rust_unoptimised" : "Rust LPRQ Unoptimised",
            "lcrq_rust_unoptimised" : "Rust LCRQ Unoptimised",
            "lprq_cpp" : "C++ LPRQ",
            "lcrq_rust" : "Rust LCRQ",
            "lprq_rust" : "Rust LPRQ",
            "moodycamel_cpp" : "moodycamel (C++)",
            "seg_queue" : "SegQueue",
            "array_queue" : "ArrayQueue",
            "atomic_queue" : "atomic_queue::Queue",
            "basic_queue" : "BasicQueue",
            "bounded_ringbuffer" : "Bounded Ringbuffer",
            "bounded_concurrent_queue" : "Bounded",
            "unbounded_concurrent_queue" : "Unbounded",
            "lf_queue" : "lf_queue::Queue",
            "lockfree_queue" : "lockfree::Queue",
            "lockfree_stack" : "lockfree::Stack",
            "scc2_queue" : "scc2::Queue",
            "scc2_stack" : "scc2::Stack",
            "scc_queue" : "scc::Queue",
            "scc_stack" : "scc::Stack",
            "boost_cpp" : "boost (C++)",
            "faaa_queue_rust" : "Rust FAAAQueue",
            "tz_queue_hp" : "TsigasZhang (HP)",
            "bbq" : "Bbq",
            "ms_queue" : "MSQueue",
            }
    # Define bounded and unbounded queue types
    bounded_queues = [
        "array_queue",
        "atomic_queue",
        "bounded_ringbuffer",
        "bounded_concurrent_queue",
        "wfqueue",
        "boost_cpp",
        "tz_queue_hp",
        "bbq",
    ]
    
    unbounded_queues = [
        'seg_queue',
        "basic_queue",
        "unbounded_concurrent_queue",
        "lf_queue",
        "lockfree_queue",
        "scc_queue",
        "scc2_queue",
        "lcrq_cpp",
        "lcrq_rust",
        "lprq_cpp",
        "lprq_rust",
        "faaa_queue_cpp",
        "faaa_queue_rust",
    ]
    
    def format_number(num):
        """Format large numbers with appropriate units (K, M, B)"""
        if num >= 1_000_000_000:
            return f"{num/1_000_000_000:.1f}B"
        elif num >= 1_000_000:
            return f"{num/1_000_000:.1f}M"
        elif num >= 1_000:
            return f"{num/1_000:.1f}K"
        else:
            return str(num)
    
    def load_data_from_folder(folder_path, dataset_name):
        """Load data from all files in a folder and label with dataset name"""
        folder = Path(folder_path)
        data_files = list(folder.glob('*'))
        
        if not data_files:
            print(f"No files found in {folder_path}")
            return None
        
        folder_data = []
        for data_file in data_files:
            try:
                df = pd.read_csv(data_file)
                df['dataset'] = dataset_name
                folder_data.append(df)
                print(f"Loaded {data_file.name}: {len(df)} rows for {dataset_name}")
            except Exception as e:
                print(f"Error reading {data_file}: {e}")
        
        if folder_data:
            combined = pd.concat(folder_data, ignore_index=True)
            print(f"Total rows for {dataset_name}: {len(combined)}")
            return combined
        return None
    
    # Load data from both folders
    youtube_data = load_data_from_folder(youtube_folder, 'soc-youtube')
    twitter_data = load_data_from_folder(twitter_folder, 'soc-twitter-2010')
    
    # Combine datasets
    all_data = []
    if youtube_data is not None:
        all_data.append(youtube_data)
    if twitter_data is not None:
        all_data.append(twitter_data)
    
    if not all_data:
        print("No data loaded successfully from either folder")
        return
    
    # Combine all dataframes
    combined_df = pd.concat(all_data, ignore_index=True)
    
    # Debug: Print unique datasets and their counts
    print(f"\nDatasets found: {combined_df['dataset'].unique()}")
    print(f"Data distribution:")
    print(combined_df['dataset'].value_counts())
    
    # Calculate mean milliseconds for each queuetype by dataset
    mean_data = combined_df.groupby(['dataset', 'Queuetype'])['Milliseconds'].mean().reset_index()
    
    # Debug: Print mean_data to see what we have
    print(f"\nMean data structure:")
    print(mean_data.head(10))
    print(f"Datasets in mean_data: {mean_data['dataset'].unique()}")
    
    # Create the plot with more space for the title
    fig, axes = plt.subplots(2, 2, figsize=(16, 12))
    
    # Create main title with graph information
    title_lines = ['BFS Benchmark Results']
    # for dataset, info in graph_info.items():
        # vertices_str = format_number(info['vertices'])
        # edges_str = format_number(info['edges'])
        # title_lines.append(f"{dataset}: {vertices_str} |V|, {edges_str} |E|")
    
    fig.suptitle('\n'.join(title_lines), fontsize=16, y=0.95)
    
    # Color palette for different queue types
    colors = plt.cm.Set3(np.linspace(0, 1, len(mean_data['Queuetype'].unique())))
    queue_colors = dict(zip(mean_data['Queuetype'].unique(), colors))
    
    # Plot data for each subplot
    subplot_configs = [
        (0, 0, 'soc-youtube', bounded_queues, 'Bounded Queues'),
        (1, 0, 'soc-youtube', unbounded_queues, 'Unbounded Queues'),
        (0, 1, 'soc-twitter-2010', bounded_queues, 'Bounded Queues'),
        (1, 1, 'soc-twitter-2010', unbounded_queues, 'Unbounded Queues')
    ]
    
    for row, col, dataset_key, queue_list, queue_type_title in subplot_configs:
        ax = axes[row, col]
        
        # Get data for this dataset
        plot_data = mean_data[mean_data['dataset'] == dataset_key]
        print(f"\nProcessing {dataset_key} - {queue_type_title}")
        print(f"Found {len(plot_data)} queue types for {dataset_key}")
        
        # Filter for the specific queue types (bounded or unbounded)
        filtered_data = plot_data[plot_data['Queuetype'].isin(queue_list)]
        print(f"After filtering for {queue_type_title.lower()}: {len(filtered_data)} queue types")
        
        if len(filtered_data) == 0:
            ax.text(0.5, 0.5, f'No {queue_type_title.lower()} data found\nfor {dataset_key}', 
                   ha='center', va='center', transform=ax.transAxes)
            # Include graph info in subplot title
            if dataset_key in graph_info:
                info = graph_info[dataset_key]
                vertices_str = format_number(info['vertices'])
                edges_str = format_number(info['edges'])
                ax.set_title(f'{dataset_key}\n{queue_type_title}\n({vertices_str} |V|, {edges_str} |E|)')
            else:
                ax.set_title(f'{dataset_key}\n{queue_type_title}')
            continue
        
        # Create bar plot
        queue_types = filtered_data['Queuetype'].values
        milliseconds = filtered_data['Milliseconds'].values
        
        print(f"Plotting queue types: {queue_types}")
        print(f"Values: {milliseconds}")
        
        bars = ax.bar(range(len(queue_types)), milliseconds, 
                     color=[queue_colors[qt] for qt in queue_types])
        
        # Customize the subplot
        if row == 0:
            # Include graph info in subplot title
            if dataset_key in graph_info:
                info = graph_info[dataset_key]
                vertices_str = format_number(info['vertices'])
                edges_str = format_number(info['edges'])
                ax.set_title(f'{dataset_key}\n(|V| = {vertices_str}, |E| = {edges_str})\n{queue_type_title}', fontsize='16')
            else:
                ax.set_title(f'{dataset_key}\n{queue_type_title}')
        else:
            ax.set_title(f'{queue_type_title}', fontsize='16')
        
        # if row == 1:
        #     ax.set_yscale('log')
        ax.set_xlabel('Queue Type', fontsize='14')
        ax.set_ylabel('Mean Milliseconds', fontsize='14')
        ax.set_xticks(range(len(queue_types)))
        translated_names = [name_translator.get(name, name) for name in queue_types]
        ax.set_xticklabels(translated_names, rotation=45, ha='right', fontsize="12")
        ax.tick_params(axis='y', labelsize=12)
        
        # Add value labels on bars
        # for bar, value in zip(bars, milliseconds):
        #     height = bar.get_height()
        #     ax.text(bar.get_x() + bar.get_width()/2., height,
        #            f'{value:.0f}', ha='center', va='bottom')
        
        # Add grid for better readability
        ax.grid(True, alpha=0.3, axis='y')
    
    # Adjust layout to prevent overlap
    plt.tight_layout(rect=[0, 0.03, 1, 0.92])  # Leave space for the main title
    
    # Print summary statistics
    print("\nSummary Statistics:")
    print("="*50)
    summary = combined_df.groupby(['dataset', 'Queuetype'])['Milliseconds'].agg(['mean', 'std', 'count'])
    print(summary)
    
    # Show the plot
    # plt.show()
    
    # Optionally save the plot
    plt.savefig('bfs_benchmark_results.pdf', dpi=1200, bbox_inches='tight')

def main():
    """
    Main function to run the benchmark plotter.
    Accepts two folder paths as command line arguments or prompts for input.
    """
    import sys
    
    if len(sys.argv) >= 3:
        youtube_folder = sys.argv[1]
        twitter_folder = sys.argv[2]
    else:
        print("Please provide paths to your data folders:")
        youtube_folder = input("Enter path to soc-youtube data folder: ").strip()
        twitter_folder = input("Enter path to soc-twitter-2010 data folder: ").strip()
    
    # Validate folder paths
    if not os.path.exists(youtube_folder):
        print(f"Error: YouTube folder '{youtube_folder}' does not exist")
        return
    
    if not os.path.exists(twitter_folder):
        print(f"Error: Twitter folder '{twitter_folder}' does not exist")
        return
    
    print(f"Processing YouTube data from: {youtube_folder}")
    print(f"Processing Twitter data from: {twitter_folder}")
    
    plot_bfs_benchmarks(youtube_folder, twitter_folder)

if __name__ == "__main__":
    main()
