{{- define "backup.backupEnvironment" -}}
# awscli writes to ~/.aws/cli/cache/
# gsutil writes to ~/.gsutil/
# azcopy writes to ~/.azcopy/
- name: HOME
  value: /tmp
{{- if hasPrefix "s3" (toString .config.location) }}
- name: BUCKET
  value: {{ .config.s3.bucket }}
{{- end }}
{{- if hasPrefix "gcs" (toString .config.location) }}
- name: BUCKET
  value: {{ .config.gcs.bucket }}
{{- end }}
{{- if hasPrefix "azure" (toString .config.location) }}
- name: ACCOUNT
  value: {{ .config.azure.account }}
- name: CONTAINER
  value: {{ .config.azure.container }}
- name: SAS
  value: {{ .config.azure.sas }}
{{- end }}
{{- if hasPrefix "r2" (toString .config.location) }}
- name: BUCKET
  value: {{ .config.r2.bucket }}
- name: R2_ENDPOINT_URL
  value: {{ .config.r2.endpoint_url }}
- name: AWS_ACCESS_KEY_ID
  valueFrom:
    secretKeyRef:
      name: r2-credentials
      key: access-key-id
- name: AWS_SECRET_ACCESS_KEY
  valueFrom:
    secretKeyRef:
      name: r2-credentials
      key: secret-access-key
{{- end }}
{{- if hasPrefix "scw_s3" (toString .config.location) }}
- name: AWS_ACCESS_KEY_ID
  value: {{ .config.scw_s3.access_key }}
- name: AWS_SECRET_ACCESS_KEY
  value: {{ .config.scw_s3.secret_key }}
- name: AWS_DEFAULT_REGION
  value: {{ .config.scw_s3.region }}
- name: BUCKET
  value: {{ .config.scw_s3.bucket }}
- name: ENDPOINT_URL
  value: {{ .config.scw_s3.endpoint_url }}
{{- end }}
{{- if hasPrefix "oci" (toString .config.location) }}
- name: ACCESS_URI
  value: {{ .config.oci.access_uri }}
- name: ENDPOINT
  value: {{ .config.oci.endpoint }}
{{- end }}
- name: SUB_DIR
  value: e{{ .era }}
{{- end -}}
