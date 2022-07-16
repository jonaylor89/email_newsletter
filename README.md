
# Email Newsletter

As the blog author,
I want to send an email to all my confirmed subscribers,
So that I can notify them when new content is published.

### Improvements

* What happens if a user tries to subscribe twice? Make sure that they receive two confirmation emails;

* What happens if a user clicks on a confirmation link twice?

* What happens if the subscription token is well-formatted but non-existent?

* Add validation on the incoming token, we are currently passing the raw user input straight into a query (thanks sqlx for protecting us from SQL injections <3);

* Use a proper templating solution for our emails (e.g. tera);
Anything that comes to your mind!

1. Security
Our POST /newsletters endpoint is unprotected - anyone can fire a request to it and broadcast to our entire audience, unchecked.

2. You Only Get One Shot
As soon you hit POST /newsletters, your content goes out to your entire mailing list. No chance to edit or review it in draft mode before giving the green light for publishing.

3. Performance
We are sending emails out one at a time.
We wait for the current one to be dispatched successfully before moving on to the next in line.
This is not a massive issue if you have 10 or 20 subscribers, but it becomes noticeable shortly afterwards: latency is going to be horrible for newsletters with a sizeable audience.

4. Fault Tolerance
If we fail to dispatch one email we bubble up the error using ? and return a 500 Internal Server Error to the caller.
The remaining emails are never sent, nor we retry to dispatch the failed one.

5. Retry Safety
Many things can go wrong when communicating over the network. What should a consumer of our API do if they experience a timeout or a 500 Internal Server Error when calling our service?
They cannot retry - they risk sending the newsletter issue twice to the entire mailing list.

We will look at three categories of callers:
* Other APIs (machine-to-machine);
* A person, via a browser;
* Another API, on behalf of a person.


- [x] Add a Send a newsletter issue link to the admin dashboard;

- [x] Add an HTML form at GET /admin/newsletters to submit a new issue;

- [x] Adapt POST /newsletters to process the form data:

- [x] Change the route to POST /admin/newsletters;

- [x] Migrate from 'Basic' to session-based authentication;

- [x] Use the Form extractor (application/x-www-form-urlencoded) instead of the Json extractor (application/json) to handle the request body;

- [ ] Adapt the test suite.