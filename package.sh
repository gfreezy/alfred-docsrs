#!/bin/bash -e

readonly workflow_dir="${1}"
readonly info_plist="${workflow_dir}/info.plist"

if [[ "$#" -ne 2 ]] || [[ ! -f "${info_plist}" ]]; then
  echo 'You need to give this script 2 arguments: the path to a valid workflow directory and output path.'
  exit 1
fi

readonly workflow_name="$(/usr/libexec/PlistBuddy -c 'print name' "${info_plist}")"
readonly workflow_file="${2}/${workflow_name}.alfredworkflow"

if /usr/libexec/PlistBuddy -c 'print variablesdontexport' "${info_plist}" &> /dev/null; then
  readonly workflow_dir_to_package="$(mktemp -d)"
  cp -R "${workflow_dir}/"* "${workflow_dir_to_package}"

  readonly tmp_info_plist="${workflow_dir_to_package}/info.plist"
  /usr/libexec/PlistBuddy -c 'Print variablesdontexport' "${tmp_info_plist}" | grep '    ' | sed -E 's/ {4}//' | xargs -I {} /usr/libexec/PlistBuddy -c "Set variables:'{}' ''" "${tmp_info_plist}"
else
  readonly workflow_dir_to_package="${workflow_dir}"
fi

ditto -ck "${workflow_dir_to_package}" "${workflow_file}"
echo "Exported worflow to ${workflow_file}."
