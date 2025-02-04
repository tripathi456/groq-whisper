from plyer import notification

def show_notification(title, message, timeout=3):
    """
    Display a desktop notification.

    Args:
        title (str): The title of the notification.
        message (str): The message body.
        timeout (int, optional): How long (in seconds) the notification stays on screen.
    """
    notification.notify(
        title=title,
        message=message,
        timeout=timeout
    )
