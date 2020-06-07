#!/usr/bin/env sh

# TODO: this shall not fail
# XFAIL: *

# REQUIRES: preload, shell, dynamic-shell
# RUN: %{shell} -c "%{intercept} --verbose --output %t.json -- %{shell} %s --sleep %{sleep} --true %{true} & %{sleep} 1; kill -s 15 %1; wait;"
# RUN: assert_intercepted %t.json count -ge 2
# RUN: assert_intercepted %t.json contains -program %{true}
# RUN: assert_intercepted %t.json contains -program %{sleep}

for i in "$@"
do
  case $i in
    --sleep)
      SLEEP=$2
      shift
      shift
      ;;
    --true)
      TRUE=$2
      shift
      shift
      ;;
    *)
      # unknown option
      ;;
  esac
done

echo "SLEEP     = $SLEEP"
echo "TRUE      = $TRUE"

if [ -z "$SLEEP" ]; then
  echo "SLEEP is not defined";
  exit 1;
fi

if [ -z "$TRUE" ]; then
  echo "TRUE is not defined";
  exit 1;
fi

# do the test
$TRUE
$SLEEP 5
