import { ToastProps } from "../components/toast";

export const toast = (props: Omit<ToastProps, "id">) =>
  // @ts-expect-error id comes as a number. unsure why it expects a string
  sonnerToast.custom((id) => <Toast id={id} {...props} />, {
    duration: props.duration,
  });
