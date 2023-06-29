#!/usr/bin/env sh

# RUN: cd %T; %{shell} %s %t
# RUN: %{citnames} --verbose --input %t.commands.json --output %t.compilations.json --config %t.config.json --append
# RUN: assert_compilation lll.json count -eq 0

cat > "$1.config.json" << EOF
{
  "linking": {
    "filename": "lll.json"
  }
}
EOF

cat > "$1.commands.json" << EOF
EOF
