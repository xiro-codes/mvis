import re

with open('src/bin/wallpaper.rs', 'r') as f:
    content = f.read()

# Fields to skip
skip_fields = ['params.particle_count', 'params.particle_types', 'params.gravity_wells', 'params.mvis_repeat_count']

def replacer(match):
    full_match = match.group(0)
    field = match.group(1)
    range_str = match.group(2)
    if field in skip_fields:
        return full_match
    return f'normalized_slider_f32(ui, &mut {field}, {range_str})'

# This matches ui.add(egui::Slider::new(&mut params.field, range));
pattern = r'ui\.add\(egui::Slider::new\(&mut (params\.[a-zA-Z0-9_]+), ([-0-9\.]+\.\.=[-0-9\.]+)\)\)'
content = re.sub(pattern, replacer, content)

# Now manually fix type proportions
prop_pattern = r'ui\.add\(\s*egui::Slider::new\(&mut (params\.type_proportions\[i\]), (0\.0\.\.=5\.0)\)\s*\.text\((format!\("Type \{\}", i\))\),\s*\);'
def prop_replacer(match):
    return f'ui.horizontal(|ui| {{ ui.label({match.group(3)}); normalized_slider_f32(ui, &mut {match.group(1)}, {match.group(2)}); }});'
content = re.sub(prop_pattern, prop_replacer, content)

with open('src/bin/wallpaper.rs', 'w') as f:
    f.write(content)
