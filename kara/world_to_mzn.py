import argparse
import json
import sys
import xml.etree.ElementTree as ET


def parse_kara_world(filename, max_states, max_steps):
    # Parse the XML file
    tree = ET.parse(filename)
    root = tree.getroot()

    # Dimensions: 0-indexed in XML, map to Width and Height 
    width = int(root.attrib["sizex"])
    height = int(root.attrib["sizey"])

    # Initialize the 2D Grid filled with Ground ('G')
    grid = [["G" for _ in range(width)] for _ in range(height)]

    # Parse Wall Points -> Trees ('T') 
    walls = root.find("XmlWallPoints")
    if walls is not None:
        for point in walls.findall("XmlPoint"):
            x = int(point.attrib["x"])
            y = int(point.attrib["y"])
            if 0 <= x < width and 0 <= y < height:
                grid[y][x] = "T"

    # Parse Painted Field Points -> End Destination 
    # Note: Converting 0-indexed XML coordinates to 1-indexed MiniZinc coordinates
    end_row, end_col = 1, 1
    painted = root.find("XmlPaintedfieldPoints")
    if painted is not None:
        point = painted.find("XmlPoint")
        if point is not None:
            end_col = int(point.attrib["x"]) + 1
            end_row = int(point.attrib["y"]) + 1

    # Parse Kara position and direction -> Start Configuration 
    start_row, start_col = 1, 1
    start_dir = "North"

    # Direction mapping: 0=North, rotating counter-clockwise
    dir_map = {0: "North", 1: "West", 2: "South", 3: "East"}

    kara_list = root.find("XmlKaraList")
    if kara_list is not None:
        kara = kara_list.find("XmlKara")
        if kara is not None:
            start_col = int(kara.attrib["x"]) + 1
            start_row = int(kara.attrib["y"]) + 1
            dir_val = int(kara.attrib["direction"])
            start_dir = dir_map.get(dir_val, "North")

    # Assemble into the MiniZinc data structure
    mzn_data = {
        "Height": height,
        "Width": width,
        "Grid": grid,
        "StartRow": start_row,
        "StartColumn": start_col,
        "StartDirection": start_dir,
        "EndRow": end_row,
        "EndColumn": end_col,
        "MaxStates": max_states,
        "MaxSteps": max_steps,
    }

    return mzn_data


def main():
    parser = argparse.ArgumentParser(
        description="Convert Kara XML world file to MiniZinc input JSON."
    )
    parser.add_argument(
        "filename", type=str, help="Path to the Kara .world XML file"
    )
    parser.add_argument(
        "max_states", type=int, help="Maximum number of automaton states"
    )
    parser.add_argument(
        "max_steps", type=int, help="Maximum number of steps for execution"
    )

    args = parser.parse_args()

    try:
        mzn_json_data = parse_kara_world(
            args.filename, args.max_states, args.max_steps
        )
        # Direct output to stdout
        json.dump(mzn_json_data, sys.stdout, indent=2)
    except Exception as e:
        print(f"Error parsing file: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
