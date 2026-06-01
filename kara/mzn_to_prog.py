import argparse
import json
import math
import sys
import xml.etree.ElementTree as ET


def main():
    parser = argparse.ArgumentParser(
        description="Convert MiniZinc solution JSON to a Kara XML program."
    )
    parser.add_argument(
        "filename", type=str, help="Path to the MiniZinc solution JSON file"
    )
    args = parser.parse_args()

    # Load JSON data
    try:
        with open(args.filename, "r") as f:
            data = json.load(f)
    except Exception as e:
        print(f"Error reading JSON file: {e}", file=sys.stderr)
        sys.exit(1)

    # Safely extract internal JSON structure if wrapped in MiniZinc output format
    if "output" in data and "json" in data["output"]:
        mzn_json = data["output"]["json"]
    else:
        mzn_json = data

    try:
        used_states = mzn_json["UsedStates"]
        conditions = mzn_json["Condition"]
        yes_actions = mzn_json["YesAction"]
        no_actions = mzn_json["NoAction"]
        yes_nexts = mzn_json["YesNext"]
        no_nexts = mzn_json["NoNext"]
    except KeyError as e:
        print(
            f"Error: Missing expected MiniZinc variable in JSON: {e}",
            file=sys.stderr,
        )
        sys.exit(1)

    # Helper function to extract strings from MiniZinc Enum JSON representation
    def get_val(item):
        if isinstance(item, dict) and "e" in item:
            return item["e"]
        return str(item)

    # Map MiniZinc enums to Kara identifiers [cite: 5, 43, 44, 45]
    sensor_map = {
        "TreeFront": "treeFront",
        "TreeLeft": "treeLeft",
        "TreeRight": "treeRight",
        "Leaf": "onLeaf",
    }

    action_map = {
        "GoForward": "move",
        "TurnLeft": "turnLeft",
        "TurnRight": "turnRight",
        "Stay": None,  # No command block element created
    }

    # Set up layout names: Active states (S1, S2...) + Stop state [cite: 32, 34]
    state_names = [f"S{i}" for i in range(1, used_states + 1)] + ["Stop"]
    total_states = len(state_names)

    # Calculate dimensions for a grid layout as square as possible
    cols = math.ceil(math.sqrt(total_states))

    state_coords = {}
    for idx, name in enumerate(state_names):
        r = idx // cols
        c = idx % cols
        # Place centered inside a 100x100 block
        x = c * 100 + 50.0
        y = r * 100 + 50.0
        state_coords[name] = (x, y)

    # Build XML Elements Tree [cite: 32]
    root = ET.Element("XmlStateMachines", version="KaraX 1.0 kara")
    machine = ET.SubElement(
        root, "XmlStateMachine", actor="Kara", startState="S1"
    )

    # 1. Generate States [cite: 32, 34]
    for name in state_names:
        x, y = state_coords[name]
        is_final = "true" if name == "Stop" else "false"
        state_el = ET.SubElement(
            machine,
            "XmlState",
            finalstate=is_final,
            name=name,
            x=f"{x:.1f}",
            y=f"{y:.1f}",
        )
        sensors_el = ET.SubElement(state_el, "XmlSensors")

        if name != "Stop":
            state_idx = int(name[1:]) - 1
            mzn_cond = get_val(conditions[state_idx])
            kara_sensor = sensor_map.get(mzn_cond, "treeFront")
            ET.SubElement(sensors_el, "XmlSensor", name=kara_sensor)

    # 2. Generate Transitions [cite: 34, 38]
    for i in range(1, used_states + 1):
        state_name = f"S{i}"
        state_idx = i - 1

        mzn_cond = get_val(conditions[state_idx])
        kara_sensor = sensor_map.get(mzn_cond, "treeFront")

        # YES Transition (SensorValue = 1) [cite: 34, 40]
        yes_next_val = yes_nexts[state_idx]
        to_state_yes = f"S{yes_next_val}" if yes_next_val > 0 else "Stop"
        yes_act = get_val(yes_actions[state_idx])
        kara_act_yes = action_map.get(yes_act)

        # 'from' is a reserved keyword in Python, passed via dict unpacking
        trans_yes = ET.SubElement(
            machine, "XmlTransition", **{"from": state_name, "to": to_state_yes}
        )
        s_vals_yes = ET.SubElement(trans_yes, "XmlSensorValues")
        ET.SubElement(
            s_vals_yes, "XmlSensorValue", name=kara_sensor, value="1"
        )
        cmds_yes = ET.SubElement(trans_yes, "XmlCommands")
        if kara_act_yes:
            ET.SubElement(cmds_yes, "XmlCommand", name=kara_act_yes)

        # NO Transition (SensorValue = 2) [cite: 36, 38]
        no_next_val = no_nexts[state_idx]
        to_state_no = f"S{no_next_val}" if no_next_val > 0 else "Stop"
        no_act = get_val(no_actions[state_idx])
        kara_act_no = action_map.get(no_act)

        trans_no = ET.SubElement(
            machine, "XmlTransition", **{"from": state_name, "to": to_state_no}
        )
        s_vals_no = ET.SubElement(trans_no, "XmlSensorValues")
        ET.SubElement(s_vals_no, "XmlSensorValue", name=kara_sensor, value="2")
        cmds_no = ET.SubElement(trans_no, "XmlCommands")
        if kara_act_no:
            ET.SubElement(cmds_no, "XmlCommand", name=kara_act_no)

    # 3. Append Default Sensor Definitions [cite: 43, 44, 45]
    sensor_defs = [
        ("Baum vorne?", "treeFront", "treeFront"),
        ("Baum links?", "treeLeft", "treeLeft"),
        ("Baum rechts?", "treeRight", "treeRight"),
        ("Pilz vorne?", "mushroomFront", "mushroomFront"),
        ("Kleeblatt unten?", "onLeaf", "onLeaf"),
    ]
    for desc, ident, name in sensor_defs:
        ET.SubElement(
            root,
            "XmlSensorDefinition",
            description=desc,
            identifier=ident,
            name=name,
        )

    # Format indentation for clean layout
    ET.indent(root, space="    ", level=0)

    # Output directly to stdout with headers [cite: 32]
    sys.stdout.write('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n')
    xml_str = ET.tostring(root, encoding="utf-8").decode("utf-8")
    sys.stdout.write(xml_str)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
