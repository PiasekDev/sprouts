#!/usr/bin/env bash
set -euo pipefail

NAMESPACE=sprouts

kubectl get storageclass standard >/dev/null

kubectl apply -f k8s/00-namespace.yaml
kubectl apply -f k8s/01-configmap.yaml
kubectl apply -f k8s/02-secret.yaml
kubectl apply -f k8s/03-serviceaccount.yaml
kubectl apply -f k8s/05-networkpolicy.yaml

kubectl apply -f k8s/10-database.yaml
kubectl rollout status statefulset/database -n "$NAMESPACE" --timeout=180s

kubectl delete job backend-migrate -n "$NAMESPACE" --ignore-not-found=true
kubectl apply -f k8s/15-backend-migration-job.yaml
kubectl wait --for=condition=complete job/backend-migrate -n "$NAMESPACE" --timeout=180s

kubectl apply -f k8s/20-backend.yaml
kubectl apply -f k8s/30-frontend.yaml
kubectl apply -f k8s/40-ingress.yaml

kubectl rollout status deployment/backend -n "$NAMESPACE" --timeout=180s
kubectl rollout status deployment/frontend -n "$NAMESPACE" --timeout=180s

kubectl get all -n "$NAMESPACE"
kubectl get ingress,pvc,networkpolicy -n "$NAMESPACE"
