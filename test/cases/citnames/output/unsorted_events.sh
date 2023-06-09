#!/usr/bin/env sh

# RUN: cd %T; %{shell} %s %t
# RUN: %{citnames} --verbose --input %t.commands.json --output-compile %t.compilations.json
# RUN: assert_compilation %t.compilations.json count -eq 1

cat << EOF > "$1.commands.json"
{"rid":"5208588700335725496","terminated":{"status":"0"},"timestamp":"2023-05-26T11:42:25.573991Z"}
{"rid":"5982815742339840829","started":{"execution":{"executable":"/usr/lib/gcc/x86_64-linux-gnu/11/collect2","arguments":["/usr/lib/gcc/x86_64-linux-gnu/11/collect2"],"working_dir":"example/build","environment":{}},"pid":13896,"ppid":13868},"timestamp":"2023-05-26T11:42:25.580038Z"}
{"rid":"16827070368060185859","started":{"execution":{"executable":"/usr/bin/ld","arguments":["/usr/bin/ld"],"working_dir":"example/build","environment":{}},"pid":13904,"ppid":13896},"timestamp":"2023-05-26T11:42:25.585572Z"}
{"rid":"16827070368060185859","terminated":{"status":"0"},"timestamp":"2023-05-26T11:42:25.638394Z"}
{"rid":"5982815742339840829","terminated":{"status":"0"},"timestamp":"2023-05-26T11:42:25.639796Z"}
{"rid":"11620369640675796770","terminated":{"status":"0"},"timestamp":"2023-05-26T11:42:25.641203Z"}
{"rid":"3208622825367537157","terminated":{"status":"0"},"timestamp":"2023-05-26T11:42:25.642386Z"}
{"rid":"3208622825367537157","started":{"execution":{"executable":"/usr/bin/make","arguments":["make","-f","../Makefile"],"working_dir":"example/build","environment":{}},"pid":13860,"ppid":13847},"timestamp":"2023-05-26T11:42:25.322304Z"}
{"rid":"11620369640675796770","started":{"execution":{"executable":"/usr/bin/g++","arguments":["g++","../main.cpp"],"working_dir":"example/build","environment":{}},"pid":13868,"ppid":13860},"timestamp":"2023-05-26T11:42:25.332432Z"}
{"rid":"1195915605071429231","started":{"execution":{"executable":"/usr/lib/gcc/x86_64-linux-gnu/11/cc1plus","arguments":["/usr/lib/gcc/x86_64-linux-gnu/11/cc1plus"],"working_dir":"example/build","environment":{}},"pid":13876,"ppid":13868},"timestamp":"2023-05-26T11:42:25.345192Z"}
{"rid":"1195915605071429231","terminated":{"status":"0"},"timestamp":"2023-05-26T11:42:25.563234Z"}
{"rid":"5208588700335725496","started":{"execution":{"executable":"/usr/bin/as","arguments":["as"],"working_dir":"example/build","environment":{}},"pid":13888,"ppid":13868},"timestamp":"2023-05-26T11:42:25.568801Z"}
EOF