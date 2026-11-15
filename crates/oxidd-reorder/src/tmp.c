/* Go through all blocks in a specific sequence and find best
   position for each of them
*/
static BddTree *reorder_sift_seq(BddTree *t, BddTree **seq, int num) {
  BddTree *this;
  int n;

  if (t == NULL)
    return t;

  for (n = 0; n < num; n++) {
    long c2, c1 = clock();

    if (verbose > 1) {
      printf("Sift ");
      if (reorder_filehandler)
        reorder_filehandler(stdout, seq[n]->id);
      else
        printf("%d", seq[n]->id);
      printf(": ");
    }

    reorder_sift_bestpos(seq[n], num / 2);

    if (verbose > 1)
      printf("\n> %d nodes", reorder_nodenum());

    c2 = clock();
    if (verbose > 1)
      printf(" (%.1f sec)\n", (float)(c2 - c1) / CLOCKS_PER_SEC);
  }

  /* Find first block */
  for (this = t; this->prev != NULL; this = this->prev)
    /* nil */;

  return this;
}
