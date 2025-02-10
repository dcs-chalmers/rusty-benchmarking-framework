import matplotlib.pyplot as plt
import pandas as pd
import sys
import os

def load_csv(filepath):
    try:
        df = pd.read_csv(filepath)
        return df
    except Exception as e:
        print(f"Error loading CSV:  {e}")
        sys.exit(1)


def plot_graph(df):
    columns = ["Throughput", "Consumers"]
    num_plots = len(columns)
    fig, axes = plt.subplots(num_plots, 1, figsize=(10, 5 * num_plots))
    
    for i, col in enumerate(columns):
        if col in df.columns:
            axes[i].plot(df.index, df[col], marker='o', label=col)
            axes[i].set_title(f'Line Plot for {col}')
            axes[i].set_xlabel("Iterations")
            axes[i].set_ylabel(col)
            axes[i].legend()
            axes[i].grid()
        else:
            print(f"Column '{col}' not found in CSV file.")
    
    plt.tight_layout()
    plt.show()


def calc_avreage(df, output_file):
    avg = df.groupby("Consumers")["Throughput"].mean().reset_index()

    if os.path.exists(output_file):
        avg.to_csv(output_file, mode='a', header=False, index=False)
    else:
        avg.to_csv(output_file, index=False)
    print(f"Averaged throughput data appended to {output_file}")


if __name__ == "__main__":
    file_path = input("Enter the CSV file path: ")
    df = load_csv(file_path)
    output_file = input("Enter output file name: ")
    calc_avreage(df, output_file)
    df_out = load_csv(output_file)
    print("CSV File Loaded Successfully! Generating Graphs...")
    plot_graph(df)

    