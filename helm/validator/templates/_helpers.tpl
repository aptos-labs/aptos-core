{{/* vim: set filetype=mustache: */}}
{{/*
Expand the name of the chart.
*/}}
{{- define "diem-validator.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "diem-validator.fullname" -}}
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
{{- define "diem-validator.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Common labels
*/}}
{{- define "diem-validator.labels" -}}
helm.sh/chart: {{ include "diem-validator.chart" . }}
{{ include "diem-validator.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end -}}

{{/*
Selector labels
*/}}
{{- define "diem-validator.selectorLabels" -}}
app.kubernetes.io/part-of: {{ include "diem-validator.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{/*
Create the name of the service account to use
*/}}
{{- define "diem-validator.serviceAccountName" -}}
{{- if .Values.serviceAccount.create -}}
    {{ default (include "diem-validator.fullname" .) .Values.serviceAccount.name }}
{{- else -}}
    {{ default "default" .Values.serviceAccount.name }}
{{- end -}}
{{- end -}}

{{- define "backup.fullname" -}}
{{ include "diem-validator.fullname" . }}
{{- end -}}

{{- define "backup.labels" -}}
{{ include "diem-validator.labels" . }}
{{- end -}}

{{- define "backup.selectorLabels" -}}
{{ include "diem-validator.selectorLabels" . }}
{{- end -}}

{{- define "backup.serviceAccount" -}}
{{ include "diem-validator.fullname" . }}-backup
{{- end -}}

{{- define "backup.backupService" -}}
{{ include "diem-validator.fullname" . }}-fullnode:6186
{{- end -}}

{{- define "backup.persistentVolumeClaim" -}}
{{ include "diem-validator.fullname" . }}-validator-e{{ .Values.chain.era }}
{{- end -}}

{{- define "backup.pushGateway" -}}
http://{{ include "diem-validator.fullname" . }}-pushgateway:9091
{{- end -}}

{{- define "backup.loggingAddress" -}}
{{ include "diem-validator.fullname" . }}-logging:5044
{{- end -}}
