{{/* vim: set filetype=mustache: */}}
{{/*
Expand the name of the chart.
*/}}
{{- define "aptos-validator.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "aptos-validator.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
{{- end -}}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "aptos-validator.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Common labels
*/}}
{{- define "aptos-validator.labels" -}}
helm.sh/chart: {{ include "aptos-validator.chart" . }}
{{ include "aptos-validator.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end -}}

{{/*
Selector labels
*/}}
{{- define "aptos-validator.selectorLabels" -}}
app.kubernetes.io/part-of: {{ include "aptos-validator.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{/*
Create the name of the service account to use
*/}}
{{- define "aptos-validator.serviceAccountName" -}}
{{- if .Values.serviceAccount.create -}}
    {{ default (include "aptos-validator.fullname" .) .Values.serviceAccount.name }}
{{- else -}}
    {{ default "default" .Values.serviceAccount.name }}
{{- end -}}
{{- end -}}

{{- define "backup.fullname" -}}
{{ include "aptos-validator.fullname" . }}
{{- end -}}

{{- define "backup.labels" -}}
{{ include "aptos-validator.labels" . }}
{{- end -}}

{{- define "backup.selectorLabels" -}}
{{ include "aptos-validator.selectorLabels" . }}
{{- end -}}

{{- define "backup.serviceAccount" -}}
{{ include "aptos-validator.fullname" . }}-backup
{{- end -}}

{{- define "backup.backupService" -}}
{{ include "aptos-validator.fullname" . }}-fullnode:6186
{{- end -}}

{{- define "backup.persistentVolumeClaim" -}}
{{ include "aptos-validator.fullname" . }}-validator-e{{ .Values.chain.era }}
{{- end -}}

{{- define "backup.pushGateway" -}}
http://{{ include "aptos-validator.fullname" . }}-pushgateway:9091
{{- end -}}

{{- define "backup.loggingAddress" -}}
{{ include "aptos-validator.fullname" . }}-logging:5044
{{- end -}}
