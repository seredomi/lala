import { Toast, ToastProps } from "../components/toast";
import { toast as sonnerToast } from "sonner";

export const toast = (props: Omit<ToastProps, "id">) =>
  // @ts-expect-error id comes as a number. unsure why it expects a string
  sonnerToast.custom((id) => <Toast id={id} {...props} />, {
    duration: props.duration,
  });
