import { Controller } from "./controller";

// Connects to data-controller="project-images"
export default class extends Controller {
  static targets = [
    "thumbnail",
    "screenshotPreviews",
    "screenshotPreviewTemplate",
  ];

  static values = {
    thumbnailUrl: String,
    screenshotUrls: Array,
  };

  declare readonly thumbnailTarget: HTMLButtonElement;
  declare readonly screenshotPreviewsTarget: HTMLElement;
  declare readonly screenshotPreviewTemplateTarget: HTMLTemplateElement;

  declare readonly thumbnailUrlValue: string | null;
  declare readonly screenshotUrlsValue: string[] | null;

  objectURLs: string[] = [];

  connect() {
    if (this.thumbnailUrlValue) {
      this.addThumbnailPreview(this.thumbnailUrlValue);
    }
    if (this.screenshotUrlsValue) {
      for (const url of this.screenshotUrlsValue) {
        this.addScreenshotPreview(url);
      }
    }
  }

  disconnect() {
    for (const objectURL of this.objectURLs) {
      URL.revokeObjectURL(objectURL);
    }
  }

  createObjectURL(obj: Blob | MediaSource): string {
    const url = URL.createObjectURL(obj);
    this.objectURLs.push(url);
    return url;
  }

  imageButtonClick(event: Event) {
    if (!(event.currentTarget instanceof Element)) return;
    const input = event.currentTarget.querySelector("input");
    if (event.target !== input) {
      event.preventDefault();
      input?.click();
    }
  }

  thumbnailChange(event: Event) {
    const input = event.target;
    if (!(input instanceof HTMLInputElement)) return;

    const { files } = input;
    if (files == null || files.length === 0) return;

    const file = files[0];
    const url = this.createObjectURL(file);
    this.addThumbnailPreview(url);
  }

  addThumbnailPreview(url: string) {
    this.thumbnailTarget.style.backgroundImage = `url(${url})`;

    for (const text of this.thumbnailTarget.querySelectorAll("p, svg")) {
      text.remove();
    }
  }

  screenshotsChange(event: Event) {
    const input = event.target;
    if (!(input instanceof HTMLInputElement)) return;

    const { files } = input;
    if (files == null || files.length === 0) return;

    const file = files[0];
    const url = this.createObjectURL(file);
    const screenshotPreview = this.addScreenshotPreview(url);

    // Move the file input into the container and create a fresh file input for
    // additional uploads.
    const newFileInput = input.cloneNode(true) as HTMLInputElement;
    newFileInput.value = "";
    input.after(newFileInput);
    screenshotPreview.appendChild(input);

    // Limit to 5 screenshots at most.
    while (this.screenshotPreviewsTarget.childElementCount > 5) {
      this.screenshotPreviewsTarget.lastElementChild?.remove();
    }
  }

  addScreenshotPreview(url: string): Element {
    const screenshotPreview =
      this.screenshotPreviewTemplateTarget.content.cloneNode(
        true
      ) as DocumentFragment;
    screenshotPreview.querySelector("img")!.src = url;
    this.screenshotPreviewsTarget.appendChild(screenshotPreview);
    return this.screenshotPreviewsTarget.lastElementChild!;
  }

  removeScreenshotPreview(event: Event) {
    if (!(event.target instanceof HTMLElement)) return;
    const container = event.target.closest("div");
    container?.remove();
  }
}
