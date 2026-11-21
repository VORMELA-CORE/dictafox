import { useEffect, useState } from "react";
import {
  Check,
  Cpu,
  Download,
  FolderOpen,
  HardDrive,
  RefreshCw,
  ShieldCheck,
  Trash2,
  X,
} from "lucide-react";
import { useApp } from "../hooks/useApp";
import type { Verification } from "../types";
import { formatBytes, percent } from "../utils/format";
import {
  Badge,
  Button,
  Field,
  Modal,
  Spinner,
  WarningBanner,
} from "../components/ui";

export function Models() {
  const {
    models,
    settings,
    capabilities,
    modelProgress,
    downloadModel,
    cancelModelDownload,
    deleteModel,
    verifyModel,
    selectModel,
    saveSettings,
    openModelsDir,
    refreshModels,
  } = useApp();
  const [verifyResults, setVerifyResults] = useState<Record<string, Verification>>(
    {},
  );
  const [verifying, setVerifying] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [customPath, setCustomPath] = useState(settings?.customModelPath ?? "");
  const [customError, setCustomError] = useState<string | null>(null);
  const [savingPath, setSavingPath] = useState(false);

  useEffect(() => {
    setCustomPath(settings?.customModelPath ?? "");
  }, [settings?.customModelPath]);

  if (!settings) return null;

  const activeModel = models.find((model) => model.active);
  const activeSpec = models.find((model) => model.id === settings.model);
  const installed = models.filter((model) => model.installed);

  const runVerify = async (modelId: string) => {
    setVerifying(modelId);
    const verification = await verifyModel(modelId);
    setVerifying(null);
    if (verification) {
      setVerifyResults((current) => ({ ...current, [modelId]: verification }));
    }
  };

  const saveCustomPath = async () => {
    setCustomError(null);
    setSavingPath(true);
    const saved = await saveSettings({
      ...settings,
      customModelPath: customPath.trim() || null,
    });
    setSavingPath(false);
    if (!saved) {
      setCustomError(
        "That model path could not be used. Choose an existing ggml model file.",
      );
    }
  };

  const applyCustomPathFile = async () => {
    if (!customPath.trim()) {
      setCustomError("Enter the path to a ggml-*.bin model first.");
      return;
    }
    await saveCustomPath();
  };

  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>Speech models</h1>
          <p className="page-subtitle">
            Models run entirely on this device through whisper.cpp. Downloads
            start only when you ask.
          </p>
        </div>
        <div className="page-actions">
          <Button
            variant="ghost"
            size="sm"
            icon={<RefreshCw size={15} />}
            onClick={() => void refreshModels()}
          >
            Refresh
          </Button>
          <Button
            variant="secondary"
            size="sm"
            icon={<FolderOpen size={15} />}
            onClick={() => void openModelsDir()}
          >
            Open models folder
          </Button>
        </div>
      </header>

      {!installed.length ? (
        <WarningBanner title="No model installed yet">
          Download at least one model to enable dictation. Tiny (English) is
          74 MB and fast; Base is more accurate. Model files are verified with
          their published SHA-256 digest.
        </WarningBanner>
      ) : null}

      <section className="card model-summary">
        <div className="summary-item">
          <span className="summary-label">Engine</span>
          <span className="summary-value">
            {capabilities?.engine ?? "whisper.cpp"}
          </span>
        </div>
        <div className="summary-item">
          <span className="summary-label">Active model</span>
          <span className="summary-value">
            {activeModel?.label ?? activeSpec?.label ?? settings.model}
          </span>
        </div>
        <div className="summary-item">
          <span className="summary-label">Models folder</span>
          <span className="summary-value mono small">
            {capabilities?.modelsDir ?? "—"}
          </span>
        </div>
        <div className="summary-item">
          <span className="summary-label">Disk free</span>
          <span className="summary-value">
            {capabilities?.diskFreeGb != null
              ? `${capabilities.diskFreeGb.toFixed(1)} GB`
              : "unknown"}
          </span>
        </div>
      </section>

      <div className="model-grid">
        {models.map((model) => {
          const progress = modelProgress[model.id];
          const verifyingThis = verifying === model.id;
          const verification = verifyResults[model.id];
          const downloading = Boolean(progress);
          return (
            <article
              className={`card model-card${model.active ? " is-active" : ""}`}
              key={model.id}
            >
              <div className="model-card-head">
                <div>
                  <h2>{model.label}</h2>
                  <p className="model-note">{model.note}</p>
                </div>
                <div className="model-badges">
                  {model.active ? <Badge tone="accent">Active</Badge> : null}
                  {model.installed ? (
                    <Badge tone="success">
                      <Check size={11} aria-hidden="true" /> Installed
                    </Badge>
                  ) : null}
                  {model.multilingual ? (
                    <Badge>Multilingual</Badge>
                  ) : (
                    <Badge>English</Badge>
                  )}
                </div>
              </div>

              <div className="model-stats">
                <span>
                  <HardDrive size={13} aria-hidden="true" />{" "}
                  {formatBytes(model.sizeBytes)}
                </span>
                <span>
                  <Cpu size={13} aria-hidden="true" /> ~
                  {model.recommendedRamGb} GB RAM
                </span>
              </div>

              {progress ? (
                <div className="download-progress">
                  <div className="progress-head">
                    <span>
                      {progress.phase === "verifying"
                        ? "Verifying SHA-256…"
                        : progress.phase === "done"
                          ? "Finishing…"
                          : `Downloading ${percent(progress.downloaded, progress.total)}%`}
                    </span>
                    <span className="muted">
                      {formatBytes(progress.downloaded)} /{" "}
                      {formatBytes(progress.total)}
                    </span>
                  </div>
                  <div className="progress-track">
                    <span
                      style={{
                        width: `${progress.phase === "downloading" ? percent(progress.downloaded, progress.total) : 100}%`,
                      }}
                    />
                  </div>
                  {progress.phase === "downloading" ? (
                    <Button
                      size="sm"
                      variant="ghost"
                      icon={<X size={13} />}
                      onClick={() => void cancelModelDownload(model.id)}
                    >
                      Cancel
                    </Button>
                  ) : (
                    <Spinner label="Verifying model" />
                  )}
                </div>
              ) : null}

              {verification ? (
                <p
                  className={`verify-note ${verification.valid ? "is-success" : "is-error"}`}
                >
                  {verification.valid ? (
                    <ShieldCheck size={14} aria-hidden="true" />
                  ) : null}
                  {verification.message}
                </p>
              ) : null}

              <div className="model-actions">
                {!model.installed ? (
                  <Button
                    variant="primary"
                    size="sm"
                    icon={<Download size={14} />}
                    disabled={downloading}
                    onClick={() => void downloadModel(model.id)}
                  >
                    {model.filePresent ? "Repair download" : "Download"}
                  </Button>
                ) : null}
                {model.installed && !model.active ? (
                  <Button
                    variant="primary"
                    size="sm"
                    icon={<Check size={14} />}
                    onClick={() => void selectModel(model.id)}
                  >
                    Use model
                  </Button>
                ) : null}
                {model.filePresent ? (
                  <Button
                    variant="ghost"
                    size="sm"
                    icon={<ShieldCheck size={14} />}
                    disabled={verifyingThis}
                    onClick={() => void runVerify(model.id)}
                  >
                    {verifyingThis ? "Verifying…" : "Verify"}
                  </Button>
                ) : null}
                {model.filePresent && !model.active ? (
                  <Button
                    variant="ghost"
                    size="sm"
                    icon={<Trash2 size={14} />}
                    onClick={() => setDeleteTarget(model.id)}
                  >
                    Delete
                  </Button>
                ) : null}
              </div>
            </article>
          );
        })}
      </div>

      <section className="card custom-model">
        <div className="card-heading">
          <h2>Custom model path</h2>
          <Badge>Advanced</Badge>
        </div>
        <p className="muted">
          Use a model you manage yourself, for example a fine-tuned whisper.cpp
          ggml file. DictaFox checks the ggml header before using it and never
          copies or uploads the file.
        </p>
        <div className="custom-path-row">
          <Field label="Model file" hint="Absolute path to a ggml-*.bin file.">
            <input
              type="text"
              value={customPath}
              onChange={(event) => setCustomPath(event.target.value)}
              placeholder="/home/you/models/ggml-custom.bin"
            />
          </Field>
          <Button
            variant="primary"
            onClick={() => void applyCustomPathFile()}
            disabled={savingPath}
          >
            {savingPath ? "Saving…" : "Use file"}
          </Button>
          {settings.customModelPath ? (
            <Button
              variant="ghost"
              onClick={() => {
                setCustomPath("");
                void saveSettings({ ...settings, customModelPath: null });
              }}
            >
              Clear
            </Button>
          ) : null}
        </div>
        {customError ? <p className="form-error">{customError}</p> : null}
      </section>

      {deleteTarget ? (
        <Modal
          title="Delete this model?"
          description="The downloaded model file will be removed from disk. You can download it again at any time."
          onClose={() => setDeleteTarget(null)}
          footer={
            <>
              <Button variant="ghost" onClick={() => setDeleteTarget(null)}>
                Cancel
              </Button>
              <Button
                variant="danger"
                icon={<Trash2 size={14} />}
                onClick={async () => {
                  await deleteModel(deleteTarget);
                  setDeleteTarget(null);
                }}
              >
                Delete model
              </Button>
            </>
          }
        />
      ) : null}
    </div>
  );
}
