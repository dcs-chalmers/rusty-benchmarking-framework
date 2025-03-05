import matplotlib.pyplot as plt
import pandas as pd
import sys
import os
import glob

def load_all_csv(folder_path):  
    folder_path = os.path.abspath(folder_path) 
    all_files = glob.glob(os.path.join(folder_path, "**", "*"), recursive=True)  

    if not all_files:
        print(f"No files found in the folder: {folder_path}")
        sys.exit(1)

    df_list = []
    for file in all_files:
        try:
            df = pd.read_csv(file)  
            df_list.append(df)
            print(f"Loaded: {file}")  
        except pd.errors.ParserError:
            print(f"Skipping (not a CSV file): {file}")
        except Exception as e:
            print(f"Error loading {file}: {e}")

    if not df_list:
        print("No valid CSV files found.")
        sys.exit(1)

    merged_df = pd.concat(df_list, ignore_index=True)
    return merged_df

# Now plots: throughput, fairness, enqueue and dequeue vs thread count
def plot_graphs(all_dfs, labels):
    fig, axes = plt.subplots(2, 2, figsize=(15, 10)) 
    metrics = ["Throughput", "Fairness", "Enqueues", "Dequeues"]
    titles = [
        "Throughput vs. Thread Count",
        "Fairness vs. Thread Count",
        "Number of Enqueues vs. Thread Count",
        "Number of Dequeues vs. Thread Count"
    ]
    
    for df, label in zip(all_dfs, labels):
        df = df.sort_values(by="Thread Count")  
        df["Thread Count"] = df["Thread Count"].astype(int)

        for i, metric in enumerate(metrics):
            row, col = divmod(i, 2)
            if metric in df.columns:
                axes[row, col].plot(df["Thread Count"], df[metric], marker='o', linestyle='-', label=label)

    for i, ax in enumerate(axes.flat):
        ax.set_title(titles[i])
        ax.set_xlabel("Thread Count")
        ax.set_ylabel(metrics[i])
        ax.legend()
        ax.grid()

    plt.tight_layout()
    plt.show()

def calc_average(df):
    df["Thread Count"] = pd.to_numeric(df["Thread Count"], errors='coerce')

    queue_type = df["Queuetype"].iloc[0] if "Queuetype" in df.columns else "Unknown"

    numeric_cols = df.select_dtypes(include=['number']).copy()  
    numeric_cols["Thread Count"] = df["Thread Count"]  
    avg = numeric_cols.groupby("Thread Count", as_index=False).mean()

    return avg, queue_type 

if __name__ == "__main__":
    amount = int(input("How many tests do you want to plot?: "))  
    all_dfs = []
    labels = []

    for i in range(amount):
        folder_path = input(f"Enter folder path for test {i+1}: ").strip()
    
        if not os.path.isdir(folder_path):
            print("Invalid folder path.")
            sys.exit(1)

        df = load_all_csv(folder_path)
        avg_df, queue_type = calc_average(df)  
        all_dfs.append(avg_df)
        labels.append(queue_type) 
        
    print("CSV Files Merged Successfully! Generating Graphs...")
    plot_graphs(all_dfs, labels)
