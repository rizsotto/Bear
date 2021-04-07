#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell
# RUN: %{shell} -c "%{intercept} --verbose --output %t.events.db -- %{shell} %s --sleep %{sleep} --true %{true} & %{sleep} 1; kill -15 %1; wait;"
# RUN: %{events_db} dump --path %t.events.db --output %t.json
# RUN: assert_intercepted %t.json count -eq 3
# RUN: assert_intercepted %t.json contains -program %{true}
# RUN: assert_intercepted %t.json contains -program %{sleep} -arguments %{sleep} 5

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

forward() {
  kill -15 $child;
}

trap forward SIGTERM

# do the test
$TRUE
$SLEEP 5&

child=$!
wait $child
