{{/*
Expand the name of the chart.
*/}}
{{- define "llm-registry.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "llm-registry.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "llm-registry.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "llm-registry.labels" -}}
helm.sh/chart: {{ include "llm-registry.chart" . }}
{{ include "llm-registry.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "llm-registry.selectorLabels" -}}
app.kubernetes.io/name: {{ include "llm-registry.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "llm-registry.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "llm-registry.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Create the image name
*/}}
{{- define "llm-registry.image" -}}
{{- $registry := .Values.global.imageRegistry | default .Values.image.registry -}}
{{- $repository := .Values.image.repository -}}
{{- $tag := .Values.image.tag | default .Chart.AppVersion -}}
{{- if $registry }}
{{- printf "%s/%s:%s" $registry $repository $tag }}
{{- else }}
{{- printf "%s:%s" $repository $tag }}
{{- end }}
{{- end }}

{{/*
Database host
*/}}
{{- define "llm-registry.database.host" -}}
{{- if .Values.postgresql.enabled }}
{{- printf "%s-%s" .Release.Name "postgresql" }}
{{- else }}
{{- .Values.externalDatabase.host }}
{{- end }}
{{- end }}

{{/*
Database port
*/}}
{{- define "llm-registry.database.port" -}}
{{- if .Values.postgresql.enabled }}
{{- 5432 }}
{{- else }}
{{- .Values.externalDatabase.port }}
{{- end }}
{{- end }}

{{/*
Database name
*/}}
{{- define "llm-registry.database.name" -}}
{{- if .Values.postgresql.enabled }}
{{- .Values.postgresql.auth.database }}
{{- else }}
{{- .Values.externalDatabase.database }}
{{- end }}
{{- end }}

{{/*
Database username
*/}}
{{- define "llm-registry.database.username" -}}
{{- if .Values.postgresql.enabled }}
{{- .Values.postgresql.auth.username }}
{{- else }}
{{- .Values.externalDatabase.username }}
{{- end }}
{{- end }}

{{/*
Database secret name
*/}}
{{- define "llm-registry.database.secretName" -}}
{{- if .Values.postgresql.enabled }}
{{- printf "%s-%s" .Release.Name "postgresql" }}
{{- else }}
{{- if .Values.secrets.existingSecret }}
{{- .Values.secrets.existingSecret }}
{{- else }}
{{- include "llm-registry.fullname" . }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Redis host
*/}}
{{- define "llm-registry.redis.host" -}}
{{- if .Values.redis.enabled }}
{{- printf "%s-%s-master" .Release.Name "redis" }}
{{- else }}
{{- .Values.externalRedis.host }}
{{- end }}
{{- end }}

{{/*
Redis port
*/}}
{{- define "llm-registry.redis.port" -}}
{{- if .Values.redis.enabled }}
{{- 6379 }}
{{- else }}
{{- .Values.externalRedis.port }}
{{- end }}
{{- end }}

{{/*
NATS URL
*/}}
{{- define "llm-registry.nats.url" -}}
{{- if .Values.nats.enabled }}
{{- printf "nats://%s-%s:4222" .Release.Name "nats" }}
{{- else }}
{{- .Values.externalNats.url }}
{{- end }}
{{- end }}

{{/*
Secret name
*/}}
{{- define "llm-registry.secretName" -}}
{{- if .Values.secrets.existingSecret }}
{{- .Values.secrets.existingSecret }}
{{- else }}
{{- include "llm-registry.fullname" . }}
{{- end }}
{{- end }}

{{/*
ConfigMap name
*/}}
{{- define "llm-registry.configMapName" -}}
{{- if .Values.configMap.existingConfigMap }}
{{- .Values.configMap.existingConfigMap }}
{{- else }}
{{- include "llm-registry.fullname" . }}
{{- end }}
{{- end }}
